//! THE CHORD BOOK (CHORDS > SHAPES): the owner's write-up of Scott McGill's "The Chord Book" videos, read from a file
//! kept on this computer only (chord_book.txt in Backline's folder; see data/chord_book.rs). Shown only when that
//! file is there. A string set and a chord family are picked in one row, then one of two views:
//! - CARDS: the family's voicings in his order, grouped as he moves through them, a card each (the chord on the
//!   family's root, his own name for it, the chord box and the notes with the tab under them, other names, the note).
//! - WALK: one voicing at a time on a big neck, the way he moves: each voice's dot slides along its string to the
//!   next voicing (the voices that moved lit up, a ghost where they were), the move in words, the voices' paths
//!   through his order drawn as lines (VOICE LINES), the four stations to jump between, and FROM HERE: every voicing
//!   of the family one move away (one voice moved, or two trading notes), to walk the map yourself.
//! What a family shows is worked out once (FAM), not every frame.

use super::*;
use crate::chord_book as cb;
use crate::clusters::step_tint;
use crate::theory;
use std::cell::RefCell;
use std::sync::Arc;

/// The parts of the book, as the BOOK dropdown names them.
const PARTS: [&str; 3] = ["Book I", "Augmented Scale", "Book II"];

/// Which part of the book a family is in: 0 Book I (the seven families on every string set), 1 the augmented scale
/// (its own videos), 2 a Book II chapter.
fn cb_part(name: &str) -> usize {
    if name.starts_with("Book II") {
        2
    } else if name.starts_with("Augmented") {
        1
    } else {
        0
    }
}

/// The MAP's columns: the families of Book I, then the augmented scale.
const MAP_COLS: [&str; 8] = ["Maj7", "m7", "7", "7#11", "Alt", "Dim", "Whole", "Aug"];
const MAP_TIPS: [&str; 8] = ["Major 7th", "Minor 7th", "Dominant 7th", "Dominant 7th #11", "Altered", "Diminished", "Whole tone", "Augmented scale"];

/// A family's column on the MAP (None for a Book II chapter).
fn cb_column(name: &str) -> Option<usize> {
    let starts = |p: &str| name.starts_with(p);
    Some(if starts("Major 7th") {
        0
    } else if starts("Minor 7th") {
        1
    } else if starts("Dominant 7th #11") {
        3
    } else if starts("Dominant 7th") {
        2
    } else if starts("Altered") {
        4
    } else if starts("Diminished") {
        5
    } else if starts("Whole Tone") {
        6
    } else if starts("Augmented") {
        7
    } else {
        return None;
    })
}

/// A string set as the dropdowns show it: "1 2 3 4  (D G B E)".
fn set_label(s: &cb::Set) -> String {
    if s.label.is_empty() {
        s.name.clone()
    } else {
        format!("{}  ({})", s.name, s.label)
    }
}

/// What a card shows, worked out from a voicing.
struct Card {
    grip: Option<[Option<u8>; 6]>,
    symbol: String,
    names: String,
    notes: String,
    degrees: Vec<String>,
    also: String,
    note: String,
    unsure: bool,
    outside: String,
    /// its key in the stars file (see cb::star_key)
    key: String,
}

/// A move in words without its note names, for HIDE NAMES: which voice went which way and how far
/// ("Alto F# to E (down a whole step)." becomes "Alto down a whole step.", a voice exchange loses its "(G and F#)").
fn hide_notes(s: &str) -> String {
    let core = |w: &str| w.trim_matches(|c: char| "(),.:;".contains(c)).to_string();
    let is_note = |w: &str| {
        let c = core(w);
        let mut ch = c.chars();
        matches!(ch.next(), Some('A'..='G')) && ch.all(|x| x == '#' || x == 'b')
    };
    // the punctuation a dropped word ended a phrase with (kept on the word before)
    let tail = |w: &str| -> String { w.trim_end_matches(')').chars().rev().take_while(|c| ".;:,".contains(*c)).collect::<Vec<_>>().into_iter().rev().collect() };
    let toks: Vec<&str> = s.split(' ').collect();
    let mut out: Vec<String> = Vec::new();
    let mut k = 0;
    while k < toks.len() {
        let w = toks[k];
        let pair = toks.get(k + 2).is_some_and(|x| is_note(x)) && matches!(toks.get(k + 1), Some(&"to") | Some(&"and"));
        if is_note(w) && pair {
            // "F# to E", "(G and F#)"
            let t = tail(toks[k + 2]);
            if let Some(last) = out.last_mut() {
                last.push_str(&t);
            }
            k += 3;
        } else if is_note(w) {
            k += 1;
        } else {
            out.push(w.to_string());
            k += 1;
        }
    }
    let s = out.join(" ");
    // "Alto (down a whole step)" reads as "Alto down a whole step"
    if s.contains(" (up ") || s.contains(" (down ") {
        s.replace(['(', ')'], "")
    } else {
        s
    }
}

/// HOW IT WORKS: his chain, in G melodic minor (C7#11 / F#7 altered), one note at a time.
const CHAIN_TEXT: &str = "@set 1 2 3 4 | D G B E
@family Dominant 7th #11 (his chain) | C D E F# G A Bb
E G Bb D | {Em7b5} Where it starts: Em7b5.
F# G Bb D | {Gm(maj7)} Raise the root of the Em7b5, E up to F#: Gm(maj7).
F# A Bb D | {Bbmaj7#5} Raise the root of the Gm(maj7), G up to A: Bbmaj7#5.
F# A Bb E | {F#7#9} Raise the 3rd of the Bbmaj7#5, D up to E: F#7#9.
F# G Bb E | {F#7b9} Lower the #9, A down to G: F#7b9.
";

/// The chords drawn faintly behind the one on the neck: the one it came from, and the station it grew out of.
#[derive(Clone, Copy, Default)]
struct Ghosts {
    parent: Option<[Option<u8>; 6]>,
    anchor: Option<[Option<u8>; 6]>,
}

/// What the walk's two parts (the top that stays put, the rest under it) share for a frame.
struct WalkFrame<'a> {
    book: &'a cb::Book,
    set: &'a cb::Set,
    fam: &'a cb::Family,
    f: &'a Fam,
    i: usize,
    n: usize,
    hidden: bool,
    /// where it resolves, then the passing chords into it (`n_pass`), then the rules
    rules: Vec<cb::Rule>,
    n_res: usize,
    n_pass: usize,
    /// the family's root, as its scale spells it
    root: String,
    /// each step starred (this one: `stars[i]`)
    stars: Vec<bool>,
}

/// What a frame of the walk asks for, done once it's drawn: a step to go to, a chord to try (or back from it), another
/// string set.
#[derive(Default)]
struct WalkAsk {
    go: Option<usize>,
    try_k: Option<usize>,
    back: bool,
    jump: Option<(usize, usize, Option<(usize, usize)>)>,
    /// star this chord, or take its star off
    star: bool,
}

/// Text in a slot `lines` lines high whatever its length, so what's under it stays put: each part from a new line,
/// wrapped, the last line cut at a word with "…" when there's more; all of it (`full`) under the pointer then.
fn slot_text(ui: &mut egui::Ui, lines: usize, parts: &[(&str, Color32)], full: &str) {
    let px = 2.0;
    let lh = px * 10.0;
    let w = ui.available_width() - 12.0;
    let (r, resp) = ui.allocate_exact_size(Vec2::new(w.max(60.0), lh * lines as f32), Sense::hover());
    let room = r.width() - 8.0;
    let mut rows: Vec<(String, Color32)> = Vec::new();
    for (text, col) in parts {
        for l in crate::font::keep_caps(|| crate::font::wrap(text, px, room)) {
            rows.push((l, *col));
        }
    }
    let cut = rows.len() > lines;
    if cut {
        let col = rows[lines - 1].1;
        let rest: Vec<String> = rows.drain(lines - 1..).map(|r| r.0).collect();
        rows.push((cut_words(&rest.join(" "), px, room - 6.0), col));
    }
    for (k, (l, col)) in rows.iter().enumerate() {
        let y = r.min.y + lh * (k as f32 + 0.5);
        crate::font::keep_caps(|| ptext_fit(ui.painter(), Pos2::new(r.min.x + 4.0, y), Align::Min, l, px, room, *col));
    }
    if cut {
        resp.on_hover_ui(|ui| {
            ui.set_max_width(440.0);
            ui.label(full);
        });
    }
}

/// A key on the keyboard, small, centred on `c`: "LEFT", "RIGHT", "UP" and "DOWN" an arrow on the cap, else the word.
fn keycap(p: &egui::Painter, c: Pos2, key: &str) {
    let col = pal().ink2;
    let dir = match key {
        "LEFT" => Some(Vec2::new(-1.0, 0.0)),
        "RIGHT" => Some(Vec2::new(1.0, 0.0)),
        "UP" => Some(Vec2::new(0.0, -1.0)),
        "DOWN" => Some(Vec2::new(0.0, 1.0)),
        _ => None,
    };
    let w = if dir.is_some() { 20.0 } else { crate::font::keep_caps(|| crate::font::text_w(key, 1.3)) + 10.0 };
    let r = Rect::from_center_size(c, Vec2::new(w, 16.0));
    p.rect_stroke(r, 3.0, Stroke::new(1.0_f32, col.gamma_multiply(0.8)));
    match dir {
        Some(d) => {
            let (a, b) = (c - d * 5.0, c + d * 5.0);
            let side = Vec2::new(-d.y, d.x);
            let s = Stroke::new(1.5_f32, col);
            p.line_segment([a, b], s);
            p.line_segment([b, b - d * 3.5 + side * 3.0], s);
            p.line_segment([b, b - d * 3.5 - side * 3.0], s);
        }
        None => {
            crate::font::keep_caps(|| ptext(p, c, Align::Center, key, 1.3, col));
        }
    }
}

/// What a click on a card asks for.
#[derive(Clone, Copy)]
enum CardAsk {
    Walk,
    Star,
}

/// A family worked out: its walk and its cards (in the walk's order), and each group's steps (in his order).
struct Fam {
    walk: cb::Walk,
    cards: Vec<Card>,
    groups: Vec<(String, Vec<usize>)>,
}

thread_local! {
    /// Where the neck's view starts and how many frets it shows, when a narrow window shows only some of them.
    static NECK_VIEW: std::cell::Cell<(usize, usize)> = const { std::cell::Cell::new((0, 0)) };
    /// The open family: (the book, set, family, order) it was made for.
    static FAM: RefCell<Option<(usize, usize, usize, cb::Order, Arc<Fam>)>> = const { RefCell::new(None) };
}

/// How long a move takes to slide (seconds), and how long AUTO stays on each voicing.
const SLIDE: f32 = 0.45;
const AUTO_EVERY: f32 = 2.6;
/// The voices' colours on the VOICE LINES, low to high (the neck's dots use them too).
const VOICE_COLS: [Color32; 6] = [
    Color32::from_rgb(70, 150, 255),
    Color32::from_rgb(90, 200, 120),
    Color32::from_rgb(255, 170, 60),
    Color32::from_rgb(240, 96, 96),
    Color32::from_rgb(180, 130, 255),
    Color32::from_rgb(60, 200, 200),
];

fn voice_col(i: usize, n: usize) -> Color32 {
    // bass blue, tenor green, alto orange, top red; a three-note voicing uses bass, middle and top; five or six
    // voices add purple (the middle voice) and teal (the baritone)
    let k = match (n, i) {
        (3, 2) => 3,
        (5, 2) => 4,
        (5, 3) => 2,
        (5, 4) => 3,
        (6, 1) => 5,
        (6, 2) => 1,
        (6, 3) => 4,
        (6, 4) => 2,
        (6, 5) => 3,
        _ => i,
    };
    VOICE_COLS[k.min(5)]
}

/// Lines of small text across a card `w` wide.
fn card_text(ui: &mut egui::Ui, w: f32, text: &str, col: Color32) {
    for line in crate::font::wrap(text, 1.7, w - 12.0) {
        let (r, _) = ui.allocate_exact_size(Vec2::new(w, 15.0), Sense::hover());
        // (as written: note names and degrees keep their capitals)
        crate::font::keep_caps(|| ptext(ui.painter(), Pos2::new(r.min.x + 6.0, r.center().y), Align::Min, &line, 1.7, col));
    }
}

/// Text across the page, wrapped, as written.
fn page_text(ui: &mut egui::Ui, text: &str, px: f32, col: Color32) {
    let w = ui.available_width() - 12.0;
    for line in crate::font::wrap(text, px, w) {
        let (r, _) = ui.allocate_exact_size(Vec2::new(w, px * 9.0), Sense::hover());
        crate::font::keep_caps(|| ptext(ui.painter(), Pos2::new(r.min.x + 4.0, r.center().y), Align::Min, &line, px, col));
    }
}

/// A small label inside a row of controls (no form column).
fn inline_label(ui: &mut egui::Ui, text: &str) {
    let w = crate::font::text_w(text, 1.7) + 12.0;
    let (r, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
    ptext(ui.painter(), Pos2::new(r.min.x + 4.0, r.center().y), Align::Min, text, 1.7, pal().ink2);
}

/// `text` cut at a word with "…" to fit `room` at size `px` (as it is when it fits).
fn cut_words(text: &str, px: f32, room: f32) -> String {
    crate::font::keep_caps(|| {
        if crate::font::text_w(text, px) <= room {
            return text.to_string();
        }
        let mut words: Vec<&str> = text.split(' ').collect();
        while words.len() > 1 && crate::font::text_w(&format!("{}…", words.join(" ")), px) > room {
            words.pop();
        }
        format!("{}…", words.join(" ").trim_end_matches([',', ':', ';', '.']))
    })
}

/// One quiet line (cut to fit, as written), the whole of `full` under the pointer.
fn dim_line(ui: &mut egui::Ui, short: &str, full: &str) {
    let px = 1.7;
    let w = ui.available_width() - 8.0;
    let (r, resp) = ui.allocate_exact_size(Vec2::new(w.max(60.0), px * 9.0), Sense::hover());
    let col = if resp.hovered() { pal().ink } else { pal().ink2 };
    // (cut at a word with "…" rather than shrunk: the size stays readable)
    let room = r.width() - 8.0;
    let line = cut_words(short, px, room);
    crate::font::keep_caps(|| ptext_fit(ui.painter(), Pos2::new(r.min.x + 4.0, r.center().y), Align::Min, &line, px, room, col));
    if !full.is_empty() && full != short {
        resp.on_hover_ui(|ui| {
            ui.set_max_width(440.0);
            ui.label(full);
        });
    }
}

/// A wrapping row of cards, their tops lined up (cards here are of different heights).
fn cb_grid(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    ui.add_space(6.0);
    ui.with_layout(egui::Layout::left_to_right(egui::Align::Min).with_main_wrap(true), |ui| {
        ui.spacing_mut().item_spacing = Vec2::new(22.0, 22.0);
        ui.add_space(6.0);
        add(ui);
    });
    ui.add_space(10.0);
}

/// The notes a grip sounds, low to high.
fn grip_pitches(g: &[Option<u8>; 6]) -> Vec<i32> {
    crate::chords::grip_notes(g)
}

impl App {
    /// Reads chord_book.txt once (again after SYNC brings a new one, or RELOAD).
    pub(crate) fn cb_load(&mut self) {
        if !self.cb_tried {
            self.cb_tried = true;
            self.cb_book = cb::load().map(|(b, _)| b).filter(|b| !b.sets.is_empty()).map(Arc::new);
            FAM.with(|c| *c.borrow_mut() = None);
        }
    }

    /// The voicings you starred (read from the file the first time).
    fn cb_stars(&mut self) -> &Vec<String> {
        self.cb_stars.get_or_insert_with(cb::load_stars)
    }

    /// Star a voicing, or take its star off, and save at once.
    fn cb_star_flip(&mut self, key: &str) {
        let stars = self.cb_stars.get_or_insert_with(cb::load_stars);
        match stars.iter().position(|k| k == key) {
            Some(at) => {
                stars.remove(at);
            }
            None => stars.push(key.to_string()),
        }
        cb::save_stars(stars);
    }

    /// WALK's order (see cb::Order).
    fn cb_walk_order(&self) -> cb::Order {
        if self.cb_order == 1 {
            cb::Order::ByStation
        } else {
            cb::Order::Lesson
        }
    }

    /// A family of a set worked out in WALK's `order` (made once, then kept while it's open).
    fn cb_fam(book: &Arc<cb::Book>, si: usize, fi: usize, order: cb::Order) -> Arc<Fam> {
        let key = Arc::as_ptr(book) as usize;
        if let Some(c) = FAM.with(|c| c.borrow().as_ref().filter(|x| x.0 == key && x.1 == si && x.2 == fi && x.3 == order).map(|x| x.4.clone())) {
            return c;
        }
        let set = &book.sets[si];
        let fam = &set.families[fi];
        let walk = fam.walk(&set.strings, order);
        let cards: Vec<Card> = walk
            .steps
            .iter()
            .map(|st| {
                let v = fam.voicing(st.at);
                let degrees = v.degrees(fam);
                // (his names first; the analyzer's others after, without repeating his)
                let also: Vec<String> = v.also(fam, 4).into_iter().filter(|n| !v.names.contains(n)).take(3).collect();
                Card {
                    grip: st.grip,
                    symbol: v.symbol(fam),
                    names: v.names.join(", "),
                    notes: format!("{}   ({})", v.spelled.join(" "), degrees.join(" ")),
                    degrees,
                    also: also.join(", "),
                    note: v.note.clone(),
                    unsure: v.unsure,
                    outside: v.outside(fam).join(" "),
                    key: cb::star_key(set, fam, v),
                }
            })
            .collect();
        // (the walk may play a station twice, or move it to its path: each group's voicings once, in his order)
        let groups = fam
            .groups
            .iter()
            .enumerate()
            .map(|(gi, g)| {
                let steps = (0..g.voicings.len()).filter_map(|v| walk.steps.iter().position(|s| s.at == (gi, v))).collect();
                (g.title.clone(), steps)
            })
            .collect();
        let f = Arc::new(Fam { walk, cards, groups });
        FAM.with(|c| *c.borrow_mut() = Some((key, si, fi, order, f.clone())));
        f
    }

    /// THE CHORD BOOK page.
    pub(crate) fn chord_book(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        self.cb_load();
        let Some(book) = self.cb_book.clone() else { return };
        self.cb_set = self.cb_set.min(book.sets.len() - 1);
        let set = &book.sets[self.cb_set];
        if set.families.is_empty() {
            para(ui, "This string set has no chord families yet.", pal().ink2);
            return;
        }
        self.cb_fam = self.cb_fam.min(set.families.len() - 1);
        let fam = &set.families[self.cb_fam];
        let (si, fi) = (self.cb_set, self.cb_fam);
        // one row: the part of the book; then for Book I (and the augmented scale) the string set and its family, for
        // Book II the chapter and the string sets it uses; the view; the video; read the file again
        let part = cb_part(&fam.name);
        let mut pick: Option<(usize, usize)> = None;
        ui.horizontal_wrapped(|ui| {
            let have: Vec<usize> = (0..3).filter(|p| book.sets.iter().any(|s| s.families.iter().any(|f| cb_part(&f.name) == *p))).collect();
            if have.len() > 1 {
                let names: Vec<&str> = have.iter().map(|&p| PARTS[p]).collect();
                let cur = have.iter().position(|&p| p == part).unwrap_or(0);
                if let Some(k) = dropdown(ui, "cb_part", "BOOK", &names, cur, 130.0) {
                    let p = have[k];
                    pick = book.sets.iter().enumerate().find_map(|(si, s)| s.families.iter().position(|f| cb_part(&f.name) == p).map(|fi| (si, fi)));
                }
            }
            if part == 2 {
                // a Book II chapter, then the string sets it's written on
                let mut chapters: Vec<&str> = Vec::new();
                for s in &book.sets {
                    for f in &s.families {
                        if cb_part(&f.name) == 2 && !chapters.contains(&f.name.as_str()) {
                            chapters.push(&f.name);
                        }
                    }
                }
                let labels: Vec<&str> = chapters.iter().map(|c| c.trim_start_matches("Book II:").trim()).collect();
                let cur = chapters.iter().position(|c| *c == fam.name).unwrap_or(0);
                if let Some(k) = dropdown(ui, "cb_chapter", "CHAPTER", &labels, cur, 260.0) {
                    let name = chapters[k];
                    pick = book.sets.iter().enumerate().find_map(|(si, s)| s.families.iter().position(|f| f.name == name).map(|fi| (si, fi)));
                }
                let on: Vec<(usize, usize)> =
                    book.sets.iter().enumerate().filter_map(|(si, s)| s.families.iter().position(|f| f.name == fam.name).map(|fi| (si, fi))).collect();
                let names: Vec<String> = on.iter().map(|&(si, _)| set_label(&book.sets[si])).collect();
                let refs: Vec<&str> = names.iter().map(String::as_str).collect();
                let cur = on.iter().position(|&(si, _)| si == self.cb_set).unwrap_or(0);
                if let Some(k) = dropdown(ui, "cb_set", "STRING SET", &refs, cur, 150.0) {
                    pick = Some(on[k]);
                }
            } else {
                // the string sets with this part of the book, then the families there
                let on: Vec<usize> = (0..book.sets.len()).filter(|&si| book.sets[si].families.iter().any(|f| cb_part(&f.name) == part)).collect();
                let names: Vec<String> = on.iter().map(|&si| set_label(&book.sets[si])).collect();
                let refs: Vec<&str> = names.iter().map(String::as_str).collect();
                let cur = on.iter().position(|&si| si == self.cb_set).unwrap_or(0);
                if let Some(k) = dropdown(ui, "cb_set", "STRING SET", &refs, cur, 150.0) {
                    let s = &book.sets[on[k]];
                    // (the same family there when it has it, else the same kind, else the first of this part)
                    let fi = s
                        .families
                        .iter()
                        .position(|f| f.name == fam.name)
                        .or_else(|| s.families.iter().position(|f| cb_column(&f.name).is_some() && cb_column(&f.name) == cb_column(&fam.name)))
                        .or_else(|| s.families.iter().position(|f| cb_part(&f.name) == part))
                        .unwrap_or(0);
                    pick = Some((on[k], fi));
                }
                let fams: Vec<usize> = (0..set.families.len()).filter(|&fi| cb_part(&set.families[fi].name) == part).collect();
                let names: Vec<String> = fams
                    .iter()
                    .map(|&fi| {
                        let f = &set.families[fi];
                        format!("{} in {}", f.name, theory::note_name(f.root, f.sharps))
                    })
                    .collect();
                let refs: Vec<&str> = names.iter().map(String::as_str).collect();
                let cur = fams.iter().position(|&fi| fi == self.cb_fam).unwrap_or(0);
                if let Some(k) = dropdown(ui, "cb_fam", "FAMILY", &refs, cur, 220.0) {
                    pick = Some((self.cb_set, fams[k]));
                }
            }
            ui.add_space(10.0);
            // the views as a small switch of icons, each named on hover (the owner's pick over a row of words)
            // (the switch's order on screen; cb_view keeps its old numbers: 0 cards, 1 walk, 2 how, 3 2-5-1, 4 map)
            const SHOWN: [u8; 5] = [4, 0, 1, 3, 2];
            let cur = SHOWN.iter().position(|v| *v == self.cb_view.min(4)).unwrap_or(1);
            let tips = [
                "Map: every string set and family at a glance",
                "Cards: every voicing as a card",
                "Walk: step from voicing to voicing and see the voices move",
                "2-5-1: the voicings through a 2-5-1",
                "How It Works: how the voicings are made",
            ];
            if let Some(v) = crate::icon_switch(ui, &["map", "cards", "walk", "=2-5-1", "book"], cur, &tips) {
                self.cb_view = SHOWN[v];
                self.cb_auto = None;
                self.dirty = true;
            }
            if !fam.video.is_empty() && crate::line_icon_btn(ui, "video", "The video lesson for this family, on YouTube").clicked() {
                crate::views::open_link(&format!("https://www.youtube.com/watch?v={}", fam.video));
            }
            // in a window of its own (to keep it beside the chart and the song; the main window goes back to where it
            // was), or back in (Pop In: the main window on the Chord Book again)
            if self.cb_in_window {
                if retro_btn(ui, "Pop In", false).tip("Back into the main window").clicked() {
                    self.cb_pop = false;
                    self.dirty = true;
                }
            } else if crate::line_icon_btn(ui, "mini", "Pop out into its own window").clicked() {
                self.cb_pop = true;
                self.dirty = true;
                if self.can_go(true) {
                    acts.push(Action::Back);
                }
            }
            // how the walk works: hear each voicing, hide the names
            if self.cb_view == 1 || self.cb_view == 3 {
                ui.add_space(8.0);
                if crate::check_box(ui, "SOUND", self.cb_sound).tip("Hear each voicing as you walk to it").clicked() {
                    self.cb_sound = !self.cb_sound;
                    self.dirty = true;
                }
            }
            if self.cb_view == 1 {
                if crate::check_box(ui, "HIDE NAMES", self.cb_hide)
                    .tip("Hide the chord name, the notes and the degrees: play it, say them, then SHOW NAMES")
                    .clicked()
                {
                    self.cb_hide = !self.cb_hide;
                    self.cb_shown = None;
                    self.dirty = true;
                }
                if fam.has_station_paths() {
                    ui.add_space(8.0);
                    let cur = self.cb_order.min(1) as usize;
                    let resp = dropdown(ui, "cb_order", "ORDER", &["Lesson", "By Station"], cur, 130.0);
                    if let Some(k) = resp {
                        // (the same chord, in the other order)
                        let at = Self::cb_fam(&book, self.cb_set, self.cb_fam, self.cb_walk_order()).walk.steps.get(self.cb_at).map(|s| s.at);
                        self.cb_order = k as u8;
                        let nf = Self::cb_fam(&book, self.cb_set, self.cb_fam, self.cb_walk_order());
                        self.cb_at = at.and_then(|at| nf.walk.steps.iter().position(|s| s.at == at)).unwrap_or(0);
                        self.cb_anim = None;
                        self.cb_try = None;
                        self.dirty = true;
                    }
                }
            }
        });
        if let Some((si, fi)) = pick {
            self.cb_set = si;
            self.cb_fam = fi;
            self.cb_at = 0;
            self.cb_anim = None;
            self.cb_try = None;
            self.dirty = true;
        }
        ui.add_space(4.0);
        if self.cb_view == 4 {
            self.cb_map(ui, &book);
            return;
        }
        let f = Self::cb_fam(&book, si, fi, self.cb_walk_order());
        if self.cb_view == 1 {
            self.cb_walk(ui, acts, &book, set, fam, &f);
            return;
        }
        if self.cb_view == 2 {
            self.cb_how(ui, acts);
            return;
        }
        if self.cb_view == 3 {
            self.cb_cadence(ui, acts, set);
            return;
        }
        let n = f.cards.len();
        page_text(ui, &format!("{n} voicings, all from {}.", fam.scale_text), 1.9, pal().ink);
        if !fam.about.is_empty() {
            para(ui, &fam.about, pal().ink2);
        }
        let root = fam.root;
        // STARRED: the ones you starred, at the top (each once, where the lesson first plays it)
        let stars = self.cb_stars().clone();
        let mut mine: Vec<usize> = Vec::new();
        for (_, steps) in &f.groups {
            for &k in steps {
                let key = &f.cards[k].key;
                if stars.contains(key) && !mine.iter().any(|&m| f.cards[m].key == *key) {
                    mine.push(k);
                }
            }
        }
        let mut ask: Option<(usize, CardAsk)> = None;
        egui::ScrollArea::vertical().id_salt("cb_scroll").auto_shrink([false, false]).show(ui, |ui| {
            let mut parts: Vec<(String, Vec<usize>)> = Vec::new();
            if !mine.is_empty() {
                parts.push((format!("STARRED ({})", mine.len()), mine.clone()));
            }
            parts.extend(f.groups.iter().map(|(t, s)| (t.to_uppercase(), s.clone())));
            for (title, steps) in &parts {
                if !title.is_empty() {
                    section_header(ui, title);
                }
                cb_grid(ui, |ui| {
                    for &k in steps {
                        let c = &f.cards[k];
                        if let Some(a) = self.cb_card(ui, acts, c, root, fam.sharps, stars.contains(&c.key)) {
                            ask = Some((k, a));
                        }
                    }
                });
            }
            ui.add_space(20.0);
        });
        match ask {
            Some((k, CardAsk::Walk)) => {
                // (WALK FROM HERE: open it in WALK)
                self.cb_at = k;
                self.cb_view = 1;
                self.cb_anim = None;
            }
            Some((k, CardAsk::Star)) => self.cb_star_flip(&f.cards[k].key),
            None => {}
        }
    }

    /// THE CHORD BOOK's page while it's in its own window: a line saying so, and the way to bring it back.
    pub(crate) fn cb_popped_note(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.0);
        para(ui, "The Chord Book is open in its own window.", pal().ink2);
        ui.add_space(6.0);
        if retro_btn(ui, "Pop In", false).tip("Back into the main window").clicked() {
            self.cb_pop = false;
            self.dirty = true;
        }
    }

    /// THE CHORD BOOK popped out: a window of its own (where it was last, or a good size), the whole page in it, the
    /// look's colours; closing it puts the book back in the main window.
    pub(crate) fn chord_book_window(&mut self, ctx: &egui::Context, acts: &mut Vec<Action>) {
        if !self.cb_pop {
            return;
        }
        let id = egui::ViewportId::from_hash_of("chord_book_window");
        let mut vb = egui::ViewportBuilder::default().with_title("Chord Book").with_min_inner_size([520.0, 420.0]);
        vb = match self.cb_pop_rect {
            Some([x, y, w, h]) => vb.with_position([x, y]).with_inner_size([w, h]),
            None => vb.with_inner_size([980.0, 820.0]),
        };
        ctx.show_viewport_immediate(id, vb, |ctx, class| {
            let page = |me: &mut Self, ui: &mut egui::Ui, acts: &mut Vec<Action>| {
                me.cb_in_window = true;
                me.chord_book(ui, acts);
                me.cb_in_window = false;
            };
            if class == egui::ViewportClass::Embedded {
                // (no second window here: a window inside the main one instead)
                let mut open = true;
                egui::Window::new("Chord Book").open(&mut open).default_size([900.0, 700.0]).show(ctx, |ui| page(self, ui, acts));
                if !open {
                    self.cb_pop = false;
                    self.dirty = true;
                }
                return;
            }
            egui::CentralPanel::default()
                .frame(egui::Frame::none().fill(pal().display).inner_margin(egui::Margin::same(16.0)))
                .show(ctx, |ui| page(self, ui, acts));
            let (close, outer, inner) = ctx.input(|i| (i.viewport().close_requested(), i.viewport().outer_rect, i.viewport().inner_rect));
            if close {
                self.cb_pop = false;
                self.dirty = true;
            }
            // (where it is, to open there next time)
            if let (Some(o), Some(n)) = (outer, inner) {
                let r = [o.min.x, o.min.y, n.width(), n.height()];
                if self.cb_pop_rect.is_none_or(|p| p.iter().zip(r.iter()).any(|(a, b)| (a - b).abs() > 1.0)) {
                    self.cb_pop_rect = Some(r);
                    self.dirty = true;
                }
            }
        });
        if !self.cb_pop {
            self.cb_pop_in(acts);
        }
    }

    /// The Chord Book back in the main window, and the main window on it (out of Now Playing if it's there).
    fn cb_pop_in(&mut self, acts: &mut Vec<Action>) {
        if self.art_view {
            acts.push(Action::ToggleArt);
        }
        self.chords_tab = 10;
        acts.push(Action::Section(crate::Sec::Chords));
    }

    /// One voicing's card: the shared voicing card (chord box, staff and tab, HEAR / SAVE) with his name, the notes
    /// and their degrees, the other names and the note under it, then its star and WALK FROM HERE. Returns which of
    /// those two was clicked.
    fn cb_card(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, c: &Card, root: i32, sharps: bool, starred: bool) -> Option<CardAsk> {
        let clicked = std::cell::Cell::new(None);
        let extra = |ui: &mut egui::Ui, w: f32| {
            if !c.names.is_empty() {
                card_text(ui, w, &format!("Think of it as {}", c.names), pal().accent);
            }
            card_text(ui, w, &c.notes, pal().ink2);
            if !c.also.is_empty() {
                card_text(ui, w, &format!("Also: {}", c.also), pal().ink2);
            }
            if !c.note.is_empty() {
                card_text(ui, w, &c.note, pal().ink);
            }
            if c.unsure {
                card_text(ui, w, "To check against the video.", crate::chords::CL_SEMI);
            }
            if !c.outside.is_empty() {
                card_text(ui, w, &format!("Not in the scale: {}", c.outside), crate::chords::CL_SEMI);
            }
        };
        match &c.grip {
            Some(g) => {
                let notes = grip_pitches(g);
                let tl: Vec<(i32, Color32)> = step_tint(&notes).iter().map(|(m, col)| (m.rem_euclid(12), *col)).collect();
                let tint = |pc: i32| tl.iter().find(|t| t.0 == pc).map(|t| t.1);
                // (the staff spelled the family's way: sharps in G, flats in Db)
                let keep = self.chord_sp.sharps;
                self.chord_sp.sharps = sharps;
                let more = crate::clusters::CardMore { text: Some(&extra), star: Some(starred), walk: true };
                match self.voicing_card_with(ui, acts, g, root, &c.symbol, &tint, false, false, more) {
                    crate::clusters::CardAct::Star => clicked.set(Some(CardAsk::Star)),
                    crate::clusters::CardAct::Walk => clicked.set(Some(CardAsk::Walk)),
                    _ => {}
                }
                self.chord_sp.sharps = keep;
            }
            None => {
                // (notes that don't fit the strings in that order: the card says so)
                let w = 180.0 * font::ui_scale();
                let resp = ui.allocate_ui_with_layout(Vec2::new(w, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(w);
                    let (nr, _) = ui.allocate_exact_size(Vec2::new(w, 24.0), Sense::hover());
                    ptext_fit(ui.painter(), nr.center(), Align::Center, &c.symbol, 2.0, w - 12.0, pal().ink);
                    extra(ui, w);
                    card_text(ui, w, "These notes don't fit the strings in this order.", crate::chords::CL_SEMI);
                    ui.horizontal(|ui| {
                        ui.add_space(4.0);
                        let tip = if starred { "Starred, at the top of CARDS. Click to unstar" } else { "Star it to keep it at the top of CARDS" };
                        if crate::views::star_btn(ui, starred).tip(tip).clicked() {
                            clicked.set(Some(CardAsk::Star));
                        }
                        if retro_btn(ui, "WALK FROM HERE", false).tip("Open it in WALK: see the voices move from here").clicked() {
                            clicked.set(Some(CardAsk::Walk));
                        }
                    });
                });
                ui.painter().rect_stroke(resp.response.rect.expand(4.0), 4.0, Stroke::new(1.0_f32, pal().edge));
            }
        }
        clicked.get()
    }

    /// Go to step `to` of the walk: the dots slide there from where they are, and it sounds (when SOUND is on).
    fn cb_go(&mut self, f: &Fam, to: usize, acts: &mut Vec<Action>) {
        let to = to.min(f.walk.steps.len().saturating_sub(1));
        let from_grip = f.walk.steps.get(self.cb_at).and_then(|s| s.grip);
        if to == self.cb_at && self.cb_anim.is_some() {
            return;
        }
        self.cb_anim = from_grip.map(|g| (Instant::now(), g));
        self.cb_at = to;
        self.cb_shown = None;
        self.cb_try = None;
        if self.cb_sound {
            if let Some(g) = f.walk.steps[to].grip {
                acts.push(Action::PlayNotes(grip_pitches(&g)));
            }
        }
    }

    /// WALK: one voicing at a time, the voices moving (see the module's notes).
    /// The family on `strings` with `fam`'s root and scale, and the voicing of it that plays `pcs` (group, voicing),
    /// if it does: (set, family, voicing).
    fn cb_find(book: &cb::Book, strings: &[usize], fam: &cb::Family, pcs: &[i32]) -> Option<(usize, usize, Option<(usize, usize)>)> {
        let key = |f: &cb::Family| {
            let mut s = f.scale.clone();
            s.sort();
            s.dedup();
            (f.root, s)
        };
        // (the same kind of family in another key: the scale's shape over its root)
        let shape = |f: &cb::Family| {
            let mut s: Vec<i32> = f.scale.iter().map(|p| (p - f.root).rem_euclid(12)).collect();
            s.sort();
            s.dedup();
            s
        };
        let (want, want_shape) = (key(fam), shape(fam));
        let mut same_key = None;
        let mut same_kind = None;
        for (si, s) in book.sets.iter().enumerate().filter(|(_, s)| s.strings == strings) {
            for (fi, f) in s.families.iter().enumerate() {
                if key(f) == want {
                    if let Some(&at) = f.order().iter().find(|&&at| f.voicing(at).pcs == pcs) {
                        return Some((si, fi, Some(at)));
                    }
                    same_key = same_key.or(Some((si, fi, None)));
                } else if shape(f) == want_shape {
                    same_kind = same_kind.or(Some((si, fi, None)));
                }
            }
        }
        same_key.or(same_kind)
    }

    fn cb_walk(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, book: &cb::Book, set: &cb::Set, fam: &cb::Family, f: &Arc<Fam>) {
        let n = f.walk.steps.len();
        if n == 0 {
            return;
        }
        self.cb_at = self.cb_at.min(n - 1);
        // AUTO: on to the next one every few seconds (and stop at the end)
        if let Some(t) = self.cb_auto {
            if t.elapsed().as_secs_f32() >= AUTO_EVERY {
                if self.cb_at + 1 < n {
                    self.cb_go(f, self.cb_at + 1, acts);
                    self.cb_auto = Some(Instant::now());
                } else {
                    self.cb_auto = None;
                }
            }
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
        }
        let i = self.cb_at;
        // (where it resolves first, then the passing chords into it, then the rules: one list, so a try is one index)
        let mut rules = fam.resolutions(set, &f.walk, i);
        let n_res = rules.len();
        rules.extend(fam.passing(set, &f.walk, i));
        let n_pass = rules.len() - n_res;
        rules.extend(fam.rules(&f.walk, i, &set.strings));
        let stars = self.cb_stars();
        let stars: Vec<bool> = f.cards.iter().map(|c| stars.contains(&c.key)).collect();
        let w = WalkFrame {
            book,
            set,
            fam,
            f,
            i,
            n,
            // HIDE NAMES: the names, notes and degrees stay hidden until SHOW NAMES (the move keeps its voice and interval)
            hidden: self.cb_hide && self.cb_shown != Some(i),
            rules,
            n_res,
            n_pass,
            root: fam.scale_text.split_whitespace().next().unwrap_or("").to_string(),
            stars,
        };
        let mut ask = WalkAsk::default();
        // the keys: left and right the chord before and after, up and down the stations, Esc back from a chord being tried
        let (dx, dy, esc, star) = self.cb_walk_keys(ui);
        ask.star = star;
        if dx < 0 && i > 0 {
            ask.go = Some(i - 1);
        }
        if dx > 0 && i + 1 < n {
            ask.go = Some(i + 1);
        }
        if dy != 0 {
            if let Some(k) = Self::cb_station_key(f, i, dy > 0) {
                ask.go = Some(k);
            }
        }
        ask.back = esc && self.cb_try.is_some_and(|t| t.0 == i);
        // the neck and its buttons stay put (only what's under them scrolls) when the window has the room for it
        let pinned = ui.available_height() > 560.0;
        if pinned {
            self.cb_walk_top(ui, acts, &w, &mut ask);
        }
        egui::ScrollArea::vertical().id_salt("cb_walk").auto_shrink([false, false]).show(ui, |ui| {
            if !pinned {
                self.cb_walk_top(ui, acts, &w, &mut ask);
            }
            self.cb_walk_rest(ui, &w, &mut ask);
        });
        // what was asked for
        let st = &f.walk.steps[i];
        let trying = self.cb_try.filter(|t| t.0 == i).and_then(|t| w.rules.get(t.1));
        if let Some(k) = ask.try_k {
            if self.cb_try == Some((i, k)) {
                // (the chord being tried, clicked again: back to the walk's)
                ask.back = true;
            } else {
                let now = trying.and_then(|r| r.grip).or(st.grip);
                self.cb_try = Some((i, k));
                self.cb_anim = now.map(|g| (Instant::now(), g));
                if self.cb_sound {
                    // (a passing chord is heard going into this one)
                    let r = &w.rules[k];
                    let pair = if r.kind == "Passing chord" { (r.grip, st.grip) } else { (st.grip, r.grip) };
                    if let (Some(a), Some(b)) = pair {
                        acts.push(Action::PlaySequenceAt(vec![grip_pitches(&a), grip_pitches(&b)], 0.9, "cb_move".into()));
                    }
                }
            }
        }
        if ask.back {
            // (the dots slide back from the chord being tried)
            if let Some(r) = trying {
                self.cb_anim = r.grip.map(|g| (Instant::now(), g));
            }
            self.cb_try = None;
        }
        if let Some((si, fi, at)) = ask.jump {
            self.cb_set = si;
            self.cb_fam = fi;
            self.cb_at = match (self.cb_book.clone(), at) {
                (Some(b), Some(at)) => Self::cb_fam(&b, si, fi, self.cb_walk_order()).walk.steps.iter().position(|s| s.at == at).unwrap_or(0),
                _ => 0,
            };
            self.cb_try = None;
            self.cb_anim = None;
            self.dirty = true;
            return;
        }
        if ask.star {
            self.cb_star_flip(&f.cards[i].key);
        }
        if let Some(k) = ask.go {
            self.cb_go(f, k, acts);
        }
    }

    /// The walk's top, the same height from chord to chord so the neck and its buttons stay put: where you are, the
    /// chord with its notes, the move in words (or what the chord being tried is), the TRYING bar (or what the faint
    /// dots are), the neck, and the buttons with their keys under them.
    fn cb_walk_top(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, w: &WalkFrame, ask: &mut WalkAsk) {
        let (f, i, n, hidden) = (w.f, w.i, w.n, w.hidden);
        let st = &f.walk.steps[i];
        let c = &f.cards[i];
        let v = w.fam.voicing(st.at);
        let station_no = f.walk.station_at(i);
        // where we are
        let at = match station_no {
            Some(m) => format!("STATION {}", m + 1),
            None => f.groups.iter().find(|g| g.1.contains(&i)).map(|g| g.0.to_uppercase()).unwrap_or_default(),
        };
        let head = format!("{at}  ·  {} of {}", i + 1, n);
        dim_line(ui, &head, &head);
        // the chord, its notes beside it
        let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 34.0), Sense::hover());
        let p = ui.painter_at(r);
        // (starred: a gold star after it, so nothing moves)
        let mut end = r.min.x;
        if !hidden {
            crate::font::keep_caps(|| {
                let sw = ptext(&p, Pos2::new(r.min.x + 4.0, r.center().y), Align::Min, &c.symbol, 3.2, pal().ink);
                let nw = ptext(&p, Pos2::new(r.min.x + 4.0 + sw + 22.0, r.center().y + 2.0), Align::Min, &c.notes, 1.9, pal().ink2);
                end = r.min.x + 4.0 + sw + 22.0 + nw;
            });
        }
        if w.stars[i] {
            crate::views::star(&p, Pos2::new(end + 16.0, r.center().y), 8.0, true, crate::views::GOLD);
        }
        // the move in words; with a chord being tried, what it is
        let trying = self.cb_try.filter(|t| t.0 == i).and_then(|t| w.rules.get(t.1));
        if let Some(r) = trying {
            let how = if hidden {
                hide_notes(&r.how)
            } else if r.kind == "Resolution" {
                let src = if r.given {
                    " The landing as written in the lesson."
                } else if r.from_book {
                    " A grip from the book."
                } else {
                    ""
                };
                format!("{}{src}", r.how)
            } else if r.kind == "Passing chord" {
                r.how.clone()
            } else {
                format!("{} Over {}: {}.", r.how, w.root, r.over)
            };
            let text = format!("{}: {how}", r.kind);
            slot_text(ui, 2, &[(text.as_str(), pal().ink)], &text);
        } else {
            let how = match station_no {
                Some(m) if st.how.is_empty() && i == 0 => format!("Station {}, where the walk starts.", m + 1),
                Some(m) if st.how.is_empty() => format!("Station {}, back where the walk started.", m + 1),
                Some(m) => format!("Station {}. {}", m + 1, st.how),
                None if st.how.is_empty() => "Where the walk starts.".to_string(),
                None => st.how.clone(),
            };
            let how = if hidden { hide_notes(&how) } else { how };
            // (every voice moving: the headline, then each voice)
            match how.split_once(" moves up to the next note of the chord: ").or_else(|| how.split_once(" moves down to the next note of the chord: ")) {
                Some((head, each)) => {
                    let dir = if how.contains(" moves up ") { "up" } else { "down" };
                    let headline = format!("{head} moves {dir} to its next chord tone.");
                    slot_text(ui, 2, &[(headline.as_str(), pal().accent), (each, pal().ink2)], &how);
                }
                None => slot_text(ui, 2, &[(how.as_str(), pal().accent)], &how),
            }
        }
        // the TRYING bar, or what the faint dots are
        let line = Self::cb_lineage(f, i);
        let anchor = line[0];
        let parent_g = st.from.and_then(|j| f.walk.steps[j].grip);
        let anchor_g = (anchor != i && Some(anchor) != st.from).then(|| f.walk.steps[anchor].grip).flatten();
        let (bar_w, bar_h) = (ui.available_width(), bh() + 8.0);
        ui.allocate_ui_with_layout(Vec2::new(bar_w, bar_h), egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.set_min_size(Vec2::new(bar_w, bar_h));
            if let Some(r) = trying {
                // (a band: the neck shows a chord being tried, not the walk's)
                let band = ui.max_rect();
                ui.painter().rect_filled(band, 2.0, pal().sel);
                ui.painter().rect_filled(Rect::from_min_size(band.min, Vec2::new(3.0, band.height())), 0.0, pal().accent);
                ui.add_space(8.0);
                inline_label(ui, "TRYING");
                if !hidden {
                    let nw = crate::font::keep_caps(|| crate::font::text_w(&r.name, 2.2)) + 14.0;
                    let (rr, _) = ui.allocate_exact_size(Vec2::new(nw, bh()), Sense::hover());
                    crate::font::keep_caps(|| ptext(ui.painter(), Pos2::new(rr.min.x, rr.center().y), Align::Min, &r.name, 2.2, pal().accent));
                }
                let back = if hidden { "BACK TO THE WALK".to_string() } else { format!("BACK TO {}", c.symbol) };
                let tip = format!("Back to your place in the walk ({} of {n})  (Esc)", i + 1);
                if retro_btn(ui, &back, false).tip(&tip).clicked() {
                    ask.back = true;
                }
                let (kr, _) = ui.allocate_exact_size(Vec2::new(36.0, bh()), Sense::hover());
                keycap(ui.painter(), kr.center(), "ESC");
                // (when the walk has this voicing too: go there)
                if let Some(h) = r.his.filter(|h| *h != i) {
                    ui.add_space(8.0);
                    if retro_btn(ui, "IN THE WALK", false).tip("This voicing is in the walk: go there").clicked() {
                        ask.go = Some(h);
                    }
                }
                // (on another string set: that set's family with the same scale, at this voicing when it has it)
                if r.strings != w.set.strings {
                    if let Some((si, fi, at)) = Self::cb_find(w.book, &r.strings, w.fam, &r.pcs) {
                        let name = &w.book.sets[si].name;
                        let (label, tip) = match at {
                            Some(_) => (format!("GO TO {name}"), format!("The same voicing on the {name} strings")),
                            None => (format!("OPEN {name}"), format!("This family on the {name} strings (this exact voicing isn't in it)")),
                        };
                        if retro_btn(ui, &label, false).tip(&tip).clicked() {
                            ask.jump = Some((si, fi, at));
                        }
                    }
                }
            } else if parent_g.is_some() || anchor_g.is_some() {
                // the ghosts' legend: one short line, the whole of it under the pointer
                let mut short = Vec::new();
                let mut full = String::new();
                if let Some(j) = st.from {
                    short.push(format!("Faint: {}", f.cards[j].symbol));
                    full += &format!("The faint dots are the chord it came from, {}.", f.cards[j].symbol);
                }
                if anchor_g.is_some() {
                    let a = Self::cb_anchor_name(f, anchor);
                    short.push(format!("Fainter: {a}"));
                    full += &format!(" The fainter ones are {}, {}, the grip it grew out of.", a, f.cards[anchor].symbol);
                }
                dim_line(ui, &short.join("   ·   "), full.trim());
            }
        });
        // the neck, the voices sliding; a chord being tried in place of the walk's, the walk's faint behind it
        let t = self.cb_anim.map(|(t0, _)| (t0.elapsed().as_secs_f32() / SLIDE).min(1.0)).unwrap_or(1.0);
        if t < 1.0 {
            ui.ctx().request_repaint();
        } else {
            self.cb_anim = None;
        }
        let ease = 1.0 - (1.0 - t).powi(3);
        let from = self.cb_anim.map(|a| a.1);
        if let Some(r) = trying {
            Self::cb_neck(ui, &r.strings, r.grip, from, ease, &r.degrees, &r.spelled, &r.moved, Ghosts { parent: st.grip, anchor: None }, hidden);
        } else {
            Self::cb_neck(ui, &w.set.strings, st.grip, from, ease, &c.degrees, &v.spelled, &st.moved, Ghosts { parent: parent_g, anchor: anchor_g }, hidden);
        }
        // the buttons: where to go (each key under its button), what to hear, how to show it
        ui.add_space(6.0);
        let shown_grip = trying.and_then(|r| r.grip).or(st.grip);
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Min).with_main_wrap(true), |ui| {
            let steps = [
                ("PREVIOUS", "LEFT", "Previous chord along the line  (Left)", i.checked_sub(1)),
                ("NEXT", "RIGHT", "Next chord along the line  (Right)", (i + 1 < n).then_some(i + 1)),
            ];
            for (label, key, tip, to) in steps {
                ui.vertical(|ui| {
                    let b = retro_btn(ui, label, false).tip(tip);
                    if b.clicked() {
                        if let Some(k) = to {
                            ask.go = Some(k);
                        }
                    }
                    let (kr, _) = ui.allocate_exact_size(Vec2::new(b.rect.width(), 18.0), Sense::hover());
                    keycap(ui.painter(), kr.center(), key);
                });
            }
            ui.add_space(14.0);
            if retro_btn(ui, "HEAR", false).clicked() {
                if let Some(g) = shown_grip {
                    acts.push(Action::PlayNotes(grip_pitches(&g)));
                }
            }
            // (a rule being tried moves from the walk's voicing; a passing chord goes into it)
            let into = trying.is_some_and(|r| r.kind == "Passing chord");
            let from_g = if trying.is_some() { st.grip } else { st.from.and_then(|j| f.walk.steps[j].grip) };
            let tip = if into { "The passing chord, then this chord" } else { "The voicing it came from, then this one" };
            if retro_btn(ui, "HEAR THE MOVE", false).tip(tip).clicked() {
                let pair = if into { (shown_grip, st.grip) } else { (from_g, shown_grip) };
                if let (Some(a), Some(b)) = pair {
                    acts.push(Action::PlaySequenceAt(vec![grip_pitches(&a), grip_pitches(&b)], 0.9, "cb_move".into()));
                }
            }
            let auto = self.cb_auto.is_some();
            if retro_btn(ui, if auto { "STOP" } else { "AUTO" }, auto).tip("Walk automatically, one voicing every few seconds").clicked() {
                self.cb_auto = if auto { None } else { Some(Instant::now()) };
            }
            ui.add_space(14.0);
            ui.vertical(|ui| {
                let on = w.stars[i];
                let tip = if on {
                    "Starred, at the top of CARDS. Click to unstar  (S)"
                } else {
                    "Star this chord to keep it at the top of CARDS  (S)"
                };
                let b = crate::views::star_btn(ui, on).tip(tip);
                if b.clicked() {
                    ask.star = true;
                }
                let (kr, _) = ui.allocate_exact_size(Vec2::new(b.rect.width(), 18.0), Sense::hover());
                keycap(ui.painter(), kr.center(), "S");
            });
            if hidden {
                ui.add_space(14.0);
                if retro_btn(ui, "SHOW NAMES", false).tip("Check what you said").clicked() {
                    self.cb_shown = Some(i);
                }
            }
        });
        ui.add_space(8.0);
    }

    /// The rest of the walk, under its buttons: the stations and the line, the lesson's words on this chord, where it
    /// comes from, where it resolves, the passing chords into it, the rules, the voice lines, and every voicing one
    /// move away.
    fn cb_walk_rest(&mut self, ui: &mut egui::Ui, w: &WalkFrame, ask: &mut WalkAsk) {
        let (f, fam, set, i, hidden) = (w.f, w.fam, w.set, w.i, w.hidden);
        let st = &f.walk.steps[i];
        let c = &f.cards[i];
        let v = fam.voicing(st.at);
        let rules = &w.rules;
        let (n_res, n_pass) = (w.n_res, w.n_pass);
        let root = &w.root;
        // the stations, then the line: each station a stop, the chords built from it after it
        if let Some(k) = Self::cb_stations(ui, f, i, hidden) {
            ask.go = Some(k);
        }
        if let Some(k) = Self::cb_line(ui, f, i, hidden, &w.stars) {
            ask.go = Some(k);
        }
        ui.add_space(8.0);
        // the lesson's words on this chord: what to think of it as, its note, its other names
        if !hidden {
            if !c.names.is_empty() {
                page_text(ui, &format!("Think of it as {}", c.names), 2.0, pal().accent);
            }
            if !c.note.is_empty() {
                // its easier grips and substitutes, marked so they stand out
                let low = c.note.to_lowercase();
                if ["stand-in", "instead of", "easier", "substitut"].iter().any(|w| low.contains(w)) {
                    section_header(ui, "STAND-IN");
                }
                page_text(ui, &c.note, 1.9, pal().ink);
            }
            if !c.also.is_empty() {
                page_text(ui, &format!("Also: {}", c.also), 1.7, pal().ink2);
            }
        }
        // where it comes from: the moves back to the station (or the part) it grew out of
        let line = Self::cb_lineage(f, i);
        let anchor = line[0];
        if line.len() > 1 || st.from.is_some() {
            if let Some(k) = self.cb_path(ui, f, &line, st.from, hidden) {
                ask.go = Some(k);
            }
        }
        ui.add_space(8.0);
        // RESOLVES TO: where it lands (what the notes say first), each voice-led from this grip, to try on the neck
        if n_res > 0 {
            section_header(ui, "RESOLVES TO");
            let said = rules[..n_res].iter().filter(|r| r.stated).count();
            // the lesson's own first (its landing as written when it gives one), then the others worked out here
            ui.horizontal_wrapped(|ui| {
                for lesson in [true, false] {
                    let group: Vec<usize> = (0..n_res).filter(|&k| rules[k].stated == lesson).collect();
                    if group.is_empty() {
                        continue;
                    }
                    if said > 0 && said < n_res {
                        if !lesson {
                            ui.add_space(10.0);
                        }
                        inline_label(ui, if lesson { "LESSON" } else { "ALSO" });
                    }
                    for k in group {
                        let r = &rules[k];
                        let label = if hidden { r.over.clone() } else { r.name.clone() };
                        let mut tip = if hidden { "Try it on the neck".to_string() } else { format!("{}: try it on the neck", r.over) };
                        if r.given {
                            tip += ". The landing as written in the lesson";
                        } else if r.stated {
                            tip += ". The lesson resolves it here";
                        }
                        if r.from_book {
                            tip += ". A grip from the book";
                        }
                        if retro_btn(ui, &label, self.cb_try == Some((i, k))).tip(&tip).clicked() {
                            ask.try_k = Some(k);
                        }
                    }
                }
            });
            let dim = fam.name.to_lowercase().contains("diminished");
            let short = if fam.kind() != cb::Kind::Dominant || fam.own_roots {
                "Where the lesson resolves it.".to_string()
            } else if dim {
                "Four homes a minor 3rd apart, major or minor.".to_string()
            } else {
                "Down a 5th, or down a half step as a tritone sub, major or minor.".to_string()
            };
            let mut full = format!(
                "{short} Each voice moves to the nearest note of the new chord, on the same strings: the 7th falls a half step to the 3rd, and the 3rd holds as the 7th."
            );
            if said > 0 {
                full += " LESSON: where the lesson resolves it, with its landing chord when it gives one. ALSO: the other homes, worked out from this grip.";
            }
            dim_line(ui, &short, &full);
            ui.add_space(6.0);
        }
        // PASSING CHORDS: chords that lead into this one, each voice-led into it on the same strings
        if n_pass > 0 {
            section_header(ui, "PASSING CHORDS");
            ui.horizontal_wrapped(|ui| {
                for k in n_res..n_res + n_pass {
                    let r = &rules[k];
                    let label = if hidden { r.over.clone() } else { r.name.clone() };
                    let tip = if hidden { "Try it on the neck".to_string() } else { format!("{}: try it on the neck", r.over) };
                    if retro_btn(ui, &label, self.cb_try == Some((i, k))).tip(&tip).clicked() {
                        ask.try_k = Some(k);
                    }
                }
            });
            dim_line(
                ui,
                "Chords that lead into this one.",
                "Chords to play just before this one, on the same strings. A half step above or below: the same grip slides in. V7, V7 altered and the tritone sub: the dominant that resolves here, its 7th falling to this chord's 3rd. Dim below: a diminished 7th a half step under the root, most voices a half step away. Dim on the root: the root holds while the others move around it. HEAR THE MOVE plays the one you're trying, then this chord.",
            );
            ui.add_space(6.0);
        }
        // RULES: what the core changes make of this voicing, each to try on the neck; the rest one click away
        let others = &rules[n_res + n_pass..];
        if !others.is_empty() {
            section_header(ui, "RULES");
            let core = |kind: &str| {
                matches!(
                    kind,
                    "Core rule" | "The chain" | "Diminished to altered" | "Altered to diminished" | "Diminished to dominant" | "Voice exchange" | "Next inversion"
                )
            };
            let n_core = others.iter().filter(|r| core(r.kind)).count().max(3).min(others.len());
            let shown = if self.cb_more_rules { others.len() } else { n_core };
            for (k, r) in others.iter().enumerate().take(shown) {
                let k = k + n_res + n_pass;
                ui.horizontal(|ui| {
                    let label = if hidden { "TRY".to_string() } else { r.name.clone() };
                    if retro_btn_w(ui, &label, 170.0, self.cb_try == Some((i, k))).tip("Try it on the neck").clicked() {
                        ask.try_k = Some(k);
                    }
                    ui.add_space(6.0);
                    // the rule, then what it does on one quiet line (all of it under the pointer)
                    let how = if hidden { hide_notes(&r.how) } else { r.how.clone() };
                    let mut full = if hidden { how.clone() } else { format!("{} Over {root}: {}.", r.how, r.over) };
                    full += if r.in_scale { " In the scale." } else { " Borrowed from outside the scale." };
                    let mut kind = r.kind.to_string();
                    if !r.in_scale {
                        kind += "  ·  borrowed";
                    }
                    if r.his.is_some_and(|h| h != i) {
                        kind += "  ·  in the walk";
                    }
                    ui.vertical(|ui| {
                        let w = ui.available_width() - 8.0;
                        let (rr, _) = ui.allocate_exact_size(Vec2::new(w.max(60.0), 15.0), Sense::hover());
                        crate::font::keep_caps(|| ptext_fit(ui.painter(), Pos2::new(rr.min.x + 4.0, rr.center().y), Align::Min, &kind, 1.6, w - 8.0, pal().accent));
                        dim_line(ui, &how, &full);
                    });
                });
                ui.add_space(4.0);
            }
            if others.len() > n_core {
                let label = if self.cb_more_rules { "FEWER RULES".to_string() } else { format!("MORE RULES ({})", others.len() - n_core) };
                if retro_btn(ui, &label, false)
                    .tip("The rule of the fifth and fourth, the tritone, string transference, planing, the same notes in other grips, and other one-note changes")
                    .clicked()
                {
                    self.cb_more_rules = !self.cb_more_rules;
                }
            }
            ui.add_space(6.0);
        }
        // the voices' paths through the lesson's order
        section_header(ui, "VOICE LINES");
        if let Some(k) = Self::cb_lines(ui, f, i, v.pcs.len(), st.from, anchor) {
            ask.go = Some(k);
        }
        // every voicing one move from here
        let near = fam.neighbours(&f.walk, i, &set.strings);
        if !near.is_empty() {
            section_header(ui, "FROM HERE");
            for (k, how) in &near {
                ui.horizontal(|ui| {
                    if retro_btn_w(ui, &f.cards[*k].symbol, 200.0, false).tip("Walk there").clicked() {
                        ask.go = Some(*k);
                    }
                    ui.add_space(6.0);
                    let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), bh()), Sense::hover());
                    let how = if hidden { hide_notes(how) } else { how.clone() };
                    crate::font::keep_caps(|| {
                        ptext_fit(ui.painter(), Pos2::new(r.min.x, r.center().y), Align::Min, &how, 1.8, r.width() - 8.0, pal().ink2)
                    });
                });
            }
        }
        ui.add_space(20.0);
    }

    /// The WALK is the page on screen (so its arrow keys are its own, and Esc takes back a chord being tried).
    pub(crate) fn cb_walk_here(&self) -> bool {
        let shown = if self.cb_pop { self.cb_in_window } else { self.mode() == 0 && self.sec == Sec::Chords && self.chords_tab == 10 };
        shown && self.cb_view == 1 && self.cb_book.is_some()
    }

    /// A player command the walk keeps its key from: one on an arrow key (or S, the star), while the walk is on screen.
    pub(crate) fn cb_owns(&self, cmd: crate::prefs::Cmd) -> bool {
        if !self.cb_walk_here() {
            return false;
        }
        let key = crate::prefs::CMDS.iter().position(|c| c.cmd == cmd).and_then(|k| self.binds.get(k).copied().flatten());
        matches!(key, Some(egui::Key::ArrowLeft | egui::Key::ArrowRight | egui::Key::ArrowUp | egui::Key::ArrowDown | egui::Key::S))
    }

    /// A chord is being tried in the walk on screen (so Esc is the walk's: back to its chord).
    pub(crate) fn cb_trying_here(&self) -> bool {
        self.cb_walk_here() && self.cb_try.is_some_and(|t| t.0 == self.cb_at)
    }

    /// The walk's keys this frame, when nothing else has the keyboard: left and right (-1, 1), up and down (1, -1), Esc,
    /// and S (star it).
    fn cb_walk_keys(&self, ui: &egui::Ui) -> (i32, i32, bool, bool) {
        if !self.cb_walk_here() || self.search_focus || self.ed.id != 0 || self.panel_open() || ui.ctx().wants_keyboard_input() {
            return (0, 0, false, false);
        }
        ui.input(|inp| {
            let k = |key: egui::Key| inp.key_pressed(key) as i32;
            let plain = !inp.modifiers.command && !inp.modifiers.alt;
            (
                k(egui::Key::ArrowRight) - k(egui::Key::ArrowLeft),
                k(egui::Key::ArrowUp) - k(egui::Key::ArrowDown),
                inp.key_pressed(egui::Key::Escape),
                plain && inp.key_pressed(egui::Key::S),
            )
        })
    }

    /// Where the up and down keys go from step `i`, as the STATIONS buttons do: up, the next station; down, back to
    /// the start of this station's path when partway along it, else the station before.
    fn cb_station_key(f: &Fam, i: usize, up: bool) -> Option<usize> {
        let st = &f.walk.stations;
        if st.is_empty() {
            return None;
        }
        let stop = Self::cb_stop(f, i);
        let mut lit = stop.as_ref().map(|(r, _)| Self::cb_stop_numbers(f, r)).unwrap_or_default();
        if lit.is_empty() {
            // (not on a station's path: the last station before it)
            lit.extend((0..st.len()).rev().find(|&m| st[m] <= i));
        }
        if up {
            let m = lit.iter().max().map_or(0, |m| m + 1);
            return st.get(m).copied();
        }
        let lo = *lit.iter().min()?;
        let start = stop.map_or(st[lo], |(r, _)| r.start);
        if i > start && !f.walk.is_stop(i) {
            return Some(start);
        }
        lo.checked_sub(1).map(|m| st[m])
    }

    /// 2-5-1: a ii-V-I (or ii-V-i) in any key from this set's own grips, each chord the one nearest the chord before
    /// (the fewest voices moving the least, the top voice by step): pick where the 2 starts, step through it on the
    /// neck, hear it, swap in another 5 or 1.
    fn cb_cadence(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, set: &cb::Set) {
        const KEYS: [&str; 12] = ["C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B"];
        let own = set.families.iter().find(|f| f.kind() == cb::Kind::Major).map_or(0, |f| f.root);
        let key = self.cb_cad_key.unwrap_or(own).rem_euclid(12);
        let Some(cad) = set.cadence(key, self.cb_cad_minor, self.cb_cad_start, self.cb_cad_pick) else {
            page_text(ui, "This string set needs a minor 7th, a dominant 7th and a major 7th family for a 2-5-1.", 1.9, pal().ink2);
            return;
        };
        let mut go: Option<usize> = None;
        let mut redo = false;
        ui.horizontal_wrapped(|ui| {
            if let Some(k) = dropdown(ui, "cb_cad_key", "KEY", &KEYS, key as usize, 80.0) {
                self.cb_cad_key = Some(k as i32);
                redo = true;
            }
            ui.add_space(8.0);
            if crate::check_box(ui, "MINOR", self.cb_cad_minor).tip("A 2-5-1 into a minor chord (ii-V-i)").clicked() {
                self.cb_cad_minor = !self.cb_cad_minor;
                redo = true;
            }
            ui.add_space(8.0);
            inline_label(ui, "THE 2 STARTS ON");
            for k in 0..4 {
                if retro_btn_w(ui, &(k + 1).to_string(), 34.0, self.cb_cad_start == k).tip("Another grip for the 2 (one of the minor family's first four)").clicked() {
                    self.cb_cad_start = k;
                    redo = true;
                }
            }
            ui.add_space(8.0);
            if retro_btn(ui, "PLAY IT", false).tip("The three chords, one after another").clicked() {
                let all: Vec<Vec<i32>> = cad.steps.iter().filter_map(|s| s.grip.map(|g| grip_pitches(&g))).collect();
                acts.push(Action::PlaySequenceAt(all, 1.2, "cb_cadence".into()));
            }
        });
        if redo {
            self.cb_cad_pick = [0, 0];
            self.cb_cad_step = 0;
        }
        page_text(ui, "Each chord is the grip nearest the one before it: the fewest voices moving, the top voice by step. That's how a pianist voice-leads.", 1.7, pal().ink2);
        ui.add_space(4.0);
        // the three chords: one to show on the neck, the one before it faint behind
        let k = self.cb_cad_step.min(cad.steps.len() - 1);
        ui.horizontal_wrapped(|ui| {
            for (j, s) in cad.steps.iter().enumerate() {
                let num = ["2", "5", "1"][j];
                if retro_btn_w(ui, &format!("{num}  {}", s.name), 170.0, j == k).tip(&format!("From the {} family", s.family)).clicked() {
                    go = Some(j);
                }
            }
        });
        let st = &cad.steps[k];
        page_text(ui, &format!("{}  ({})", st.spelled.join(" "), st.degrees.join(" ")), 1.9, pal().ink2);
        let how = if st.how.is_empty() { "Where it starts.".to_string() } else { st.how.clone() };
        page_text(ui, &how, 2.0, pal().accent);
        let t = self.cb_anim.map(|(t0, _)| (t0.elapsed().as_secs_f32() / SLIDE).min(1.0)).unwrap_or(1.0);
        if t < 1.0 {
            ui.ctx().request_repaint();
        } else {
            self.cb_anim = None;
        }
        let ease = 1.0 - (1.0 - t).powi(3);
        let parent = k.checked_sub(1).and_then(|j| cad.steps[j].grip);
        Self::cb_neck(ui, &set.strings, st.grip, self.cb_anim.map(|a| a.1), ease, &st.degrees, &st.spelled, &st.moved, Ghosts { parent, anchor: None }, false);
        // another 5 or 1: the next smoothest choices
        if k > 0 && st.choices.len() > 1 {
            ui.horizontal_wrapped(|ui| {
                inline_label(ui, "OTHER CHOICES");
                for (c, name) in st.choices.iter().enumerate() {
                    if retro_btn(ui, name, self.cb_cad_pick[k - 1] == c).tip("Swap in this grip (smoothest first)").clicked() {
                        self.cb_cad_pick[k - 1] = c;
                        if k == 1 {
                            // (a new 5 means a new nearest 1)
                            self.cb_cad_pick[1] = 0;
                        }
                    }
                }
            });
        }
        if let Some(j) = go {
            self.cb_anim = st.grip.map(|g| (Instant::now(), g));
            self.cb_cad_step = j;
            if self.cb_sound {
                if let Some(g) = cad.steps[j].grip {
                    acts.push(Action::PlayNotes(grip_pitches(&g)));
                }
            }
        }
    }

    /// MAP: which families each string set has (Book I and the augmented scale as a grid, a cell's number its voicings),
    /// and the string sets each Book II chapter is written on. A click opens it in CARDS.
    fn cb_map(&mut self, ui: &mut egui::Ui, book: &cb::Book) {
        let mut go: Option<(usize, usize)> = None;
        // (how many of a family's voicings you starred)
        let stars = self.cb_stars().clone();
        let starred = |s: &cb::Set, f: &cb::Family| {
            let head = format!("{} | {} | ", s.name, f.name);
            stars.iter().filter(|k| k.starts_with(&head)).count()
        };
        egui::ScrollArea::vertical().id_salt("cb_map").auto_shrink([false, false]).show(ui, |ui| {
            page_text(ui, "Which families each string set has, and how many voicings. Click one to open it.", 1.7, pal().ink2);
            section_header(ui, "BOOK I AND THE AUGMENTED SCALE");
            let lw = book.sets.iter().map(|s| crate::font::text_w(&s.name, 1.8)).fold(0.0, f32::max) + 18.0;
            let cw = 50.0;
            ui.horizontal(|ui| {
                ui.add_space(lw);
                for (c, name) in MAP_COLS.iter().enumerate() {
                    let (r, resp) = ui.allocate_exact_size(Vec2::new(cw, bh()), Sense::hover());
                    crate::font::keep_caps(|| ptext(ui.painter(), r.center(), Align::Center, name, 1.7, pal().ink2));
                    resp.on_hover_text(MAP_TIPS[c]);
                }
            });
            for (si, s) in book.sets.iter().enumerate() {
                if !s.families.iter().any(|f| cb_column(&f.name).is_some()) {
                    continue;
                }
                ui.horizontal(|ui| {
                    let (r, resp) = ui.allocate_exact_size(Vec2::new(lw, bh()), Sense::hover());
                    let col = if si == self.cb_set { pal().accent } else { pal().ink };
                    ptext(ui.painter(), Pos2::new(r.min.x + 4.0, r.center().y), Align::Min, &s.name, 1.8, col);
                    if !s.label.is_empty() {
                        resp.on_hover_text(format!("The {} strings", s.label));
                    }
                    for c in 0..MAP_COLS.len() {
                        let fams: Vec<usize> = (0..s.families.len()).filter(|&fi| cb_column(&s.families[fi].name) == Some(c)).collect();
                        if fams.is_empty() {
                            let (r, _) = ui.allocate_exact_size(Vec2::new(cw, bh()), Sense::hover());
                            ptext(ui.painter(), r.center(), Align::Center, "·", 1.8, pal().ink2);
                            continue;
                        }
                        let count: usize = fams.iter().map(|&fi| s.families[fi].order().len()).sum();
                        let mine: usize = fams.iter().map(|&fi| starred(s, &s.families[fi])).sum();
                        let here = si == self.cb_set && fams.contains(&self.cb_fam);
                        let tip = fams
                            .iter()
                            .map(|&fi| {
                                let f = &s.families[fi];
                                let n = starred(s, f);
                                let st = if n > 0 { format!(", {n} starred") } else { String::new() };
                                format!("{} in {}: {} voicings{st}", f.name, theory::note_name(f.root, f.sharps), f.order().len())
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                        let b = retro_btn_w(ui, &count.to_string(), cw - 6.0, here).tip(&tip);
                        if mine > 0 {
                            // (some starred: a small star in its corner)
                            let c = b.rect.right_top() + Vec2::new(-7.0, 7.0);
                            crate::views::star(ui.painter(), c, 6.5, true, pal().display);
                            crate::views::star(ui.painter(), c, 5.0, true, crate::views::GOLD);
                        }
                        if b.clicked() {
                            go = Some((si, fams[0]));
                        }
                    }
                });
            }
            // Book II: each chapter, and the string sets it's written on
            let mut chapters: Vec<&str> = Vec::new();
            for s in &book.sets {
                for f in &s.families {
                    if cb_part(&f.name) == 2 && !chapters.contains(&f.name.as_str()) {
                        chapters.push(&f.name);
                    }
                }
            }
            if !chapters.is_empty() {
                section_header(ui, "BOOK II");
                // (the string sets in a column of their own on the left, so they line up; the chapter after them)
                let most = chapters
                    .iter()
                    .map(|name| book.sets.iter().filter(|s| s.families.iter().any(|f| f.name == *name)).count())
                    .max()
                    .unwrap_or(1);
                let bw = 78.0;
                for name in chapters {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                        ui.allocate_ui_with_layout(Vec2::new(most as f32 * (bw + 6.0), bh()), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.set_min_width(most as f32 * (bw + 6.0));
                            for (si, s) in book.sets.iter().enumerate() {
                                if let Some(fi) = s.families.iter().position(|f| f.name == name) {
                                    let n = s.families[fi].order().len();
                                    let here = si == self.cb_set && fi == self.cb_fam;
                                    if retro_btn_w(ui, &s.name, bw, here).tip(&format!("{n} voicings on the {} strings", s.label)).clicked() {
                                        go = Some((si, fi));
                                    }
                                }
                            }
                        });
                        ui.add_space(6.0);
                        // (the whole title, on two lines when it's long: its end tells the chapters apart)
                        let title = name.trim_start_matches("Book II:").trim();
                        ui.vertical(|ui| {
                            let lines = crate::font::wrap(title, 1.8, (ui.available_width() - 12.0).max(80.0));
                            let n = lines.len();
                            for line in lines {
                                let h = if n == 1 { bh() } else { 1.8 * 9.0 };
                                let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width().max(60.0), h), Sense::hover());
                                crate::font::keep_caps(|| ptext(ui.painter(), Pos2::new(r.min.x + 4.0, r.center().y), Align::Min, &line, 1.8, pal().ink));
                            }
                        });
                    });
                }
            }
            ui.add_space(12.0);
            if retro_btn(ui, "RELOAD THE BOOK", false).tip("Reload chord_book.txt after editing it").clicked() {
                self.cb_tried = false;
            }
            ui.add_space(20.0);
        });
        if let Some((si, fi)) = go {
            self.cb_set = si;
            self.cb_fam = fi;
            self.cb_at = 0;
            self.cb_anim = None;
            self.cb_try = None;
            self.cb_view = 0;
            self.dirty = true;
        }
    }

    /// HOW IT WORKS: the system in short (from the front of his book and the videos, in plain words), with his chain
    /// to play on the neck. The same rules show under RULES in WALK, applied to the grip you're on.
    fn cb_how(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        thread_local! {
            static CHAIN: (cb::Book, cb::Walk) = {
                let (b, _) = cb::parse(CHAIN_TEXT);
                let w = b.sets[0].families[0].walk(&b.sets[0].strings, cb::Order::Lesson);
                (b, w)
            };
        }
        // a list with its lines hanging under the text, not under the bullet
        let bullets = |ui: &mut egui::Ui, items: &[&str]| {
            let px = 1.9;
            let indent = 18.0;
            for t in items {
                let w = ui.available_width() - 12.0;
                for (k, line) in crate::font::wrap(t, px, w - indent).iter().enumerate() {
                    let (r, _) = ui.allocate_exact_size(Vec2::new(w, px * 9.0), Sense::hover());
                    if k == 0 {
                        ptext(ui.painter(), Pos2::new(r.min.x + 4.0, r.center().y), Align::Min, "•", px, pal().ink2);
                    }
                    crate::font::keep_caps(|| ptext(ui.painter(), Pos2::new(r.min.x + 4.0 + indent, r.center().y), Align::Min, line, px, pal().ink));
                }
                ui.add_space(3.0);
            }
        };
        egui::ScrollArea::vertical().id_salt("cb_how").auto_shrink([false, false]).show(ui, |ui| {
            section_header(ui, "THE IDEA");
            page_text(
                ui,
                "Each family is a scale, and each grip is four of its notes on four strings. Every new grip is a small move from one you know: one voice at a time, two voices trading places, or the whole shape sliding. Learn the moves and the grips follow.",
                1.9,
                pal().ink,
            );
            section_header(ui, "ONE NOTE CHANGES THE CHORD");
            bullets(
                ui,
                &[
                    "Lower the 5th of a minor 7th: m7b5.",
                    "Lower the 3rd of a major 7th, or raise the 7th of a minor 7th: m(maj7).",
                    "Raise the 5th of a major 7th: maj7#5.",
                    "Lower the 9th of a 9sus4 (F/G): 7b9sus4 (Fm/G).",
                    "A diminished 7th with any note down a half step: a dominant 7th.",
                ],
            );
            page_text(ui, "These chords have no family of their own here: they live inside the dominant families. In WALK, RULES shows them for the grip you're on.", 1.7, pal().ink2);
            section_header(ui, "THE RELATIVES OF A DOMINANT");
            bullets(
                ui,
                &[
                    "C7#11 (the notes of G melodic minor) holds Em7b5, Gm(maj7) and Bbmaj7#5.",
                    "C7 altered (the notes of Db melodic minor) holds Bbm7b5, C#m(maj7) and Emaj7#5.",
                    "A tritone apart: move a #11 grip a tritone and over the same bass it's altered, and back. Gm(maj7) is C7#11 and F#7 altered at once.",
                ],
            );
            section_header(ui, "THE CHAIN");
            page_text(ui, "One note at a time, every chord from G melodic minor. Each one also dresses up C7 (9, #11, 13) or F#7 altered.", 1.7, pal().ink2);
            CHAIN.with(|(b, w)| {
                let fam = &b.sets[0].families[0];
                let strings = &b.sets[0].strings;
                let n = w.steps.len();
                self.cb_demo = self.cb_demo.min(n.saturating_sub(1));
                let k = self.cb_demo;
                let mut go: Option<usize> = None;
                ui.horizontal_wrapped(|ui| {
                    for (j, st) in w.steps.iter().enumerate() {
                        let v = fam.voicing(st.at);
                        let name = v.names.first().cloned().unwrap_or_else(|| v.spelled.join(" "));
                        if retro_btn_w(ui, &format!("{}  {name}", j + 1), 150.0, j == k).tip(&v.note).clicked() {
                            go = Some(j);
                        }
                    }
                    if retro_btn(ui, "PLAY THE CHAIN", false).tip("All five, one after another").clicked() {
                        let all: Vec<Vec<i32>> = w.steps.iter().filter_map(|s| s.grip.map(|g| grip_pitches(&g))).collect();
                        acts.push(Action::PlaySequenceAt(all, 1.1, "cb_chain".into()));
                    }
                });
                let st = &w.steps[k];
                let v = fam.voicing(st.at);
                page_text(ui, &v.note, 1.9, pal().accent);
                let t = self.cb_anim.map(|(t0, _)| (t0.elapsed().as_secs_f32() / SLIDE).min(1.0)).unwrap_or(1.0);
                if t < 1.0 {
                    ui.ctx().request_repaint();
                } else {
                    self.cb_anim = None;
                }
                let ease = 1.0 - (1.0 - t).powi(3);
                let parent = k.checked_sub(1).and_then(|j| w.steps[j].grip);
                Self::cb_neck(ui, strings, st.grip, self.cb_anim.map(|a| a.1), ease, &v.degrees(fam), &v.spelled, &st.moved, Ghosts { parent, anchor: None }, false);
                page_text(ui, "The dots show each note's degree over C.", 1.7, pal().ink2);
                if let Some(j) = go {
                    self.cb_anim = st.grip.map(|g| (Instant::now(), g));
                    self.cb_demo = j;
                    if self.cb_sound {
                        if let Some(g) = w.steps[j].grip {
                            acts.push(Action::PlayNotes(grip_pitches(&g)));
                        }
                    }
                }
            });
            section_header(ui, "DIMINISHED AND ALTERED");
            bullets(
                ui,
                &[
                    "A 7b9/#9 grip from the diminished scale: lower its 13th, or raise its 5th, and it's altered.",
                    "An altered grip with its b13 raised to the 13th is diminished again.",
                ],
            );
            section_header(ui, "RESOLVING");
            bullets(
                ui,
                &[
                    "A dominant lands down a 5th (D7 to G), or, standing in as a tritone sub, down a half step (D7 to Db). Either one can be major or minor.",
                    "A diminished grip works on four dominants a minor 3rd apart, so it has four homes: from the E, G, Bb and Db dominants it goes to A, C, Eb or Gb.",
                    "The pull is in the guide tones: the 7th falls a half step to the next chord's 3rd, and the 3rd holds as its 7th.",
                    "Altered notes lean by a half step: the b9 down to the new 5th, the b13 down to the new 9th, the #9 down to the new 13th.",
                    "In WALK, Resolves To leads the grip you're on into each home on the same strings. Every voice moves to the nearest note of the new chord.",
                ],
            );
            section_header(ui, "PASSING CHORDS");
            bullets(
                ui,
                &[
                    "A passing chord comes just before a chord and leads into it. In WALK, Passing Chords works them out for the grip you're on, on the same strings.",
                    "A half step above or below: the same grip slides into the chord, every voice by a half step.",
                    "Its V7, plain or altered, or the tritone sub: the dominant that resolves to it. The 7th falls a half step to the chord's 3rd.",
                    "Diminished a half step below the root (C#dim7 into Dm7): most voices a half step away. Diminished on the root (Cdim7 into Cmaj7): the root holds and the rest close in on it.",
                ],
            );
            section_header(ui, "NEW GRIPS FROM OLD ONES");
            bullets(
                ui,
                &[
                    "Voice exchange: two voices a 2nd apart trade notes, each staying on its own string.",
                    "The next inversion: line the chord's notes up like a scale and move every voice to the next one. It works for chords that aren't stacked 3rds too (F G A C).",
                    "Planing: every voice to the next note of the scale. The shape slides and the sound stays in the family.",
                    "The rule of the fifth: the same shape a 5th away keeps much of the sound.",
                ],
            );
            section_header(ui, "OTHER STRINGS");
            bullets(
                ui,
                &[
                    "String transference: when a set has one note on an E string, that note can move to the other E. 1234 and 2346, 1235 and 2356, 1245 and 2456, 1345 and 3456.",
                    "Octave transference: move one note an octave onto another string. A Dm7 on 2345 becomes a 1245 or a 2456 grip.",
                    "Each string set rings differently: the same chord on another set has another color. In real playing you move between sets for the voice leading, the bass and the melody.",
                    "In WALK, More Rules tries both on the grip you're on, with a button to open that string set.",
                ],
            );
            section_header(ui, "MORE NOTES");
            bullets(
                ui,
                &[
                    "Add a bass on the 6th string or a top note on the 1st and four notes become five or six: Dbmaj7 over a Gb bass is Gbmaj9#11. In WALK, More Rules offers the ones that fit the grip you're on.",
                    "Any grip works in any job. A m7b5 found under a V chord is just as good as a ii.",
                    "Strike the strings in pairs, or let them ring one by one, and the same grip turns into a melody.",
                ],
            );
            section_header(ui, "THE HAND");
            page_text(
                ui,
                "Some grips barre with the 2nd or 3rd finger, or slant the 1st finger across two frets. If a stretch hurts, play it on another string set or a few notes at a time.",
                1.9,
                pal().ink,
            );
            ui.add_space(20.0);
        });
    }

    /// Step `i` and the moves back to where it grew from: the station (or the start of his part) it traces back to,
    /// first. Just `[i]` when it is one.
    fn cb_lineage(f: &Fam, i: usize) -> Vec<usize> {
        let mut v = vec![i];
        let mut k = i;
        while !f.walk.is_stop(k) {
            match f.walk.steps[k].from {
                Some(j) if j < k => {
                    v.push(j);
                    k = j;
                }
                _ => break,
            }
        }
        v.reverse();
        v
    }

    /// "Station 2", "Part 3" (by the jump buttons' numbers), or "where the walk starts".
    fn cb_anchor_name(f: &Fam, k: usize) -> String {
        if let Some(m) = f.walk.station_at(k) {
            return format!("Station {}", m + 1);
        }
        match f.walk.stops.iter().position(|r| r.contains(&k)) {
            Some(n) => format!("Part {}", n + 1),
            None => "Where the walk starts".to_string(),
        }
    }

    /// COMES FROM: the chords along the way from its station to here, each a button (the move into it on hover).
    /// Returns one clicked.
    fn cb_path(&self, ui: &mut egui::Ui, f: &Fam, line: &[usize], parent: Option<usize>, hidden: bool) -> Option<usize> {
        let mut go = None;
        let i = *line.last().unwrap_or(&0);
        // a long path keeps its start and its last few steps
        let shown: Vec<Option<usize>> = if line.len() > 7 {
            let mut v = vec![Some(line[0]), None];
            v.extend(line[line.len() - 5..].iter().map(|&k| Some(k)));
            v
        } else {
            line.iter().map(|&k| Some(k)).collect()
        };
        ui.horizontal_wrapped(|ui| {
            form_label(ui, "COMES FROM");
            for (n, k) in shown.iter().enumerate() {
                if n > 0 {
                    // an arrow between steps
                    let (r, _) = ui.allocate_exact_size(Vec2::new(18.0, bh()), Sense::hover());
                    let (a, b) = (Pos2::new(r.min.x + 3.0, r.center().y), Pos2::new(r.max.x - 4.0, r.center().y));
                    let col = pal().ink2;
                    ui.painter().line_segment([a, b], Stroke::new(1.5_f32, col));
                    ui.painter().line_segment([b, b + Vec2::new(-5.0, -4.0)], Stroke::new(1.5_f32, col));
                    ui.painter().line_segment([b, b + Vec2::new(-5.0, 4.0)], Stroke::new(1.5_f32, col));
                }
                let Some(k) = *k else {
                    page_text(ui, "…", 1.8, pal().ink2);
                    continue;
                };
                // (with HIDE NAMES on, the one you're on isn't named: the arrow points at the neck)
                let blank = hidden && k == i;
                if blank && n > 0 {
                    continue;
                }
                let sym = if blank { String::new() } else { f.cards[k].symbol.clone() };
                let label = if n == 0 && f.walk.is_stop(k) {
                    format!("{}  {}", Self::cb_anchor_name(f, k).to_uppercase(), sym).trim().to_string()
                } else {
                    sym
                };
                let tip = match f.walk.steps[k].how.as_str() {
                    "" => "Where the walk starts".to_string(),
                    h if blank => hide_notes(h),
                    h => h.to_string(),
                };
                let w = crate::font::keep_caps(|| crate::font::text_w(&label, 1.8)) + 24.0;
                if retro_btn_w(ui, &label, w.clamp(60.0, 320.0), k == i).tip(&tip).clicked() && k != i {
                    go = Some(k);
                }
            }
            // the chord it came from, when that isn't the step before it on this path (a jump back in his order)
            if let Some(j) = parent.filter(|j| !line.contains(j)) {
                ui.add_space(10.0);
                if retro_btn(ui, &format!("CAME FROM {}", f.cards[j].symbol), false).tip("The chord this one moved from").clicked() {
                    go = Some(j);
                }
            }
            // a chord on another station's path that fewer notes change from (the lesson puts this one with its station)
            if let Some(j) = f.walk.steps[i].also {
                let from = Self::cb_anchor_name(f, Self::cb_lineage(f, j)[0]);
                let label = if hidden { format!("ALSO NEAR {}", from.to_uppercase()) } else { format!("ALSO NEAR {}", f.cards[j].symbol) };
                let how = if hidden { hide_notes(&f.walk.steps[i].also_how) } else { f.walk.steps[i].also_how.clone() };
                let tip = format!(
                    "Fewer notes change from this chord, on the path from {}: {how} The lesson puts this one with its own station, so the path above is the one to learn.",
                    from.to_lowercase()
                );
                ui.add_space(10.0);
                if retro_btn(ui, &label, false).tip(&tip).clicked() {
                    go = Some(j);
                }
            }
        });
        let _ = self;
        go
    }

    /// The stop step `i` is on, and its path: the stop's steps (a station, several side by side on one bass, or where
    /// a part starts) and the last chord of its path (the one before the next stop). When the stations sit together at
    /// the start (nothing tells which one a chord is built from), only a station itself.
    fn cb_stop(f: &Fam, i: usize) -> Option<(std::ops::Range<usize>, usize)> {
        let n = f.walk.steps.len();
        if f.walk.inversions && !f.walk.paths {
            return f.walk.is_stop(i).then(|| (i..i + 1, i));
        }
        let j = f.walk.stops.iter().rposition(|r| r.start <= i)?;
        let r = f.walk.stops[j].clone();
        let end = f.walk.stops.get(j + 1).map_or(n - 1, |q| q.start.saturating_sub(1)).max(r.end - 1);
        Some((r, end))
    }

    /// The numbers on a stop (from 0): its stations', or its part's.
    fn cb_stop_numbers(f: &Fam, r: &std::ops::Range<usize>) -> Vec<usize> {
        if f.walk.inversions {
            r.clone().filter_map(|k| f.walk.station_at(k)).collect()
        } else {
            f.walk.stops.iter().position(|q| q == r).into_iter().collect()
        }
    }

    /// "station 3", "stations 3 and 4", "stations 5, 6 and 7" (from numbers counted from 0)
    fn cb_station_names(ms: &[usize]) -> String {
        let v: Vec<String> = ms.iter().map(|m| (m + 1).to_string()).collect();
        match v.as_slice() {
            [one] => format!("station {one}"),
            [rest @ .., last] => format!("stations {} and {last}", rest.join(", ")),
            [] => String::new(),
        }
    }

    /// STATIONS (or PARTS): a button for each, named; the ones on the stop whose path this chord is on lit. Returns
    /// one clicked.
    fn cb_stations(ui: &mut egui::Ui, f: &Fam, i: usize, hidden: bool) -> Option<usize> {
        let st = &f.walk.stations;
        if st.len() < 2 {
            return None;
        }
        let mut go = None;
        let lit: Vec<usize> = Self::cb_stop(f, i).map(|(r, _)| Self::cb_stop_numbers(f, &r)).unwrap_or_default();
        ui.horizontal_wrapped(|ui| {
            form_label(ui, if f.walk.inversions { "STATIONS" } else { "PARTS" });
            for (m, &s) in st.iter().enumerate() {
                let label = if hidden || !f.walk.inversions { (m + 1).to_string() } else { format!("{}  {}", m + 1, f.cards[s].symbol) };
                let tip = if !f.walk.inversions {
                    let g = f.groups.iter().find(|(_, r)| r.contains(&s)).map(|(t, _)| t.as_str()).unwrap_or("");
                    format!("Go to: {g}")
                } else if f.walk.paths && f.walk.order == cb::Order::Lesson {
                    format!("Go to where the walk turns to station {}. NEXT then walks the chords built from it", m + 1)
                } else if f.walk.paths {
                    format!("Go to station {}. NEXT then walks the chords built from it", m + 1)
                } else {
                    format!("Go to station {}", m + 1)
                };
                let w = crate::font::keep_caps(|| crate::font::text_w(&label, 1.8)) + 24.0;
                if retro_btn_w(ui, &label, w.clamp(34.0, 320.0), lit.contains(&m)).tip(&tip).clicked() {
                    go = Some(s);
                }
            }
            // (their keys)
            ui.add_space(10.0);
            let (kr, resp) = ui.allocate_exact_size(Vec2::new(48.0, bh()), Sense::hover());
            keycap(ui.painter(), kr.center() - Vec2::new(12.0, 0.0), "DOWN");
            keycap(ui.painter(), kr.center() + Vec2::new(12.0, 0.0), "UP");
            let what = if f.walk.inversions { "station" } else { "part" };
            let _ = resp.tip(format!("Up: next {what}. Down: previous {what}."));
        });
        go
    }

    /// THE LINE: every chord of the walk a dot along one line, each station a numbered stop with the chords built
    /// from it after it, a starred one with a small star over it; the path this chord is on drawn bold, this chord
    /// ringed. Hover names a dot, a click goes
    /// there. Under it, how far along its station's path this chord is.
    fn cb_line(ui: &mut egui::Ui, f: &Fam, i: usize, hidden: bool, stars: &[bool]) -> Option<usize> {
        let n = f.walk.steps.len();
        if n < 2 {
            return None;
        }
        let w = ui.available_width() - 8.0;
        let (r, resp) = ui.allocate_exact_size(Vec2::new(w.max(120.0), 42.0), Sense::click());
        let p = ui.painter_at(r);
        let is_stop = |k: usize| f.walk.is_stop(k);
        // the number on a stop's circle
        let number = |k: usize| f.walk.station_at(k).or_else(|| f.walk.stops.iter().position(|q| q.start == k)).unwrap_or(0) + 1;
        // (a stop takes three times a chord's room)
        let room: Vec<f32> = (0..n).map(|k| if is_stop(k) { 3.0 } else { 1.0 }).collect();
        let (left, right) = (r.min.x + 12.0, r.max.x - 12.0);
        let scale = (right - left) / room.iter().sum::<f32>();
        let mut xs = Vec::with_capacity(n);
        let mut acc = 0.0;
        for u in &room {
            xs.push(left + (acc + u / 2.0) * scale);
            acc += u;
        }
        // (room over the line for the stars)
        let cy = r.center().y + 4.0;
        let stop = Self::cb_stop(f, i);
        // the path this chord is on: its stop to the chord before the next stop
        let path = stop.clone().filter(|_| !f.walk.inversions || f.walk.paths).map(|(r, end)| (r.start, end));
        let on = stop.as_ref().map(|s| s.0.clone());
        p.line_segment([Pos2::new(xs[0], cy), Pos2::new(xs[n - 1], cy)], Stroke::new(2.0_f32, pal().edge));
        if let Some((a, b)) = path.filter(|(a, b)| b > a) {
            p.line_segment([Pos2::new(xs[a], cy), Pos2::new(xs[b], cy)], Stroke::new(3.0_f32, pal().accent.gamma_multiply(0.8)));
        }
        let rad = (1.5 * scale - 1.0).clamp(5.0, 9.0);
        let dot = (0.5 * scale - 0.5).clamp(1.5, 3.0);
        let mine = |k: usize| path.is_some_and(|(a, b)| k >= a && k <= b) || on.as_ref().is_some_and(|r| r.contains(&k));
        for k in 0..n {
            let c = Pos2::new(xs[k], cy);
            if is_stop(k) {
                p.circle_filled(c, rad, if mine(k) { pal().sel } else { pal().display });
                p.circle_stroke(c, rad, Stroke::new(1.5_f32, if mine(k) { pal().accent } else { pal().ink2 }));
                if rad >= 7.0 {
                    crate::font::keep_caps(|| ptext(&p, c, Align::Center, &number(k).to_string(), 1.3, pal().ink));
                }
            } else {
                p.circle_filled(c, dot, if mine(k) { pal().ink } else { pal().ink2.gamma_multiply(0.6) });
            }
            // (a starred chord: a small gold star over it)
            if stars.get(k).copied().unwrap_or(false) {
                crate::views::star(&p, Pos2::new(c.x, r.min.y + 5.0), 4.5, true, crate::views::GOLD);
            }
        }
        // this chord, and the one under the pointer
        let ring = |k: usize, col: Color32| {
            let c = Pos2::new(xs[k], cy);
            if is_stop(k) {
                p.circle_stroke(c, rad + 3.0, Stroke::new(2.0_f32, col));
            } else {
                p.circle_filled(c, 4.0, col);
                p.circle_stroke(c, 7.0, Stroke::new(1.5_f32, col));
            }
        };
        ring(i, pal().accent);
        let near = |x: f32| (0..n).min_by(|&a, &b| (xs[a] - x).abs().total_cmp(&(xs[b] - x).abs())).unwrap_or(0);
        let hover = resp.hover_pos().map(|pos| near(pos.x));
        if let Some(k) = hover.filter(|&k| k != i) {
            ring(k, pal().ink2);
        }
        let group = |k: usize| f.groups.iter().find(|g| g.1.contains(&k)).map(|g| g.0.clone()).unwrap_or_default();
        let tip = hover
            .map(|k| match f.walk.station_at(k) {
                Some(m) if hidden => format!("Station {}", m + 1),
                Some(m) => format!("Station {}: {}", m + 1, f.cards[k].symbol),
                None if hidden => group(k),
                None => format!("{}  ·  {}", f.cards[k].symbol, group(k)),
            })
            .unwrap_or_default();
        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand).tip(&tip);
        let mut go = None;
        if resp.clicked() {
            go = resp.interact_pointer_pos().map(|pos| near(pos.x));
        }
        // how far along
        if let (Some((r, end)), true) = (stop, !f.walk.inversions || f.walk.paths) {
            let last = r.end - 1;
            let built = end - last;
            let nums = Self::cb_stop_numbers(f, &r);
            let names = Self::cb_station_names(&nums);
            let mut head = names.clone();
            if let Some(c) = head.get_mut(..1) {
                c.make_ascii_uppercase();
            }
            let (it, its) = if nums.len() > 1 { ("them", "their") } else { ("it", "its") };
            // (an opening station the walk turns back to later)
            let opening = nums.first().is_some_and(|&m| f.walk.stations.get(m).is_some_and(|&b| b > i));
            let text = if !f.walk.inversions {
                format!("{} of {} in part {}.", i - r.start + 1, end - r.start + 1, nums.first().map_or(1, |m| m + 1))
            } else if i > last {
                format!("{} of {} built from {names}.", i - last, built)
            } else if opening {
                format!("{head} as the lesson first plays {it}. The chords built from {it} come at {its} later stop.")
            } else {
                match built {
                    0 => format!("{head}: nothing in the lesson is built from {it}. NEXT goes on to the next stop."),
                    1 => format!("{head}: the chord after {it} is built from {it}."),
                    _ => format!("{head}: the {built} chords after {it} are built from {it}."),
                }
            };
            dim_line(ui, &text, &text);
        } else {
            ui.allocate_exact_size(Vec2::new(10.0, 1.7 * 9.0), Sense::hover());
        }
        go
    }

    /// The neck with the voicing on it: each voice's dot (its degree inside) slides from `from` to `to` (`t` 0..1),
    /// the voices that moved in their colour; behind them, faint, the chord it came from (and fainter, the station it
    /// grew out of), with a thin line from where each moved voice was. Each voice is named at the left with its note.
    #[allow(clippy::too_many_arguments)]
    fn cb_neck(
        ui: &mut egui::Ui,
        strings: &[usize],
        to: Option<[Option<u8>; 6]>,
        from: Option<[Option<u8>; 6]>,
        t: f32,
        degrees: &[String],
        spelled: &[String],
        moved: &[usize],
        ghosts: Ghosts,
        hidden: bool,
    ) {
        const FRETS: usize = 19;
        let w = ui.available_width();
        let gap = 26.0;
        let (r, _) = ui.allocate_exact_size(Vec2::new(w, gap * 5.0 + 62.0), Sense::hover());
        // (kept inside its own box, whatever the window's width)
        let p = ui.painter_at(r);
        let n = strings.len();
        let labels: Vec<(usize, String)> = strings
            .iter()
            .enumerate()
            .map(|(vi, &s)| {
                let note = if hidden { String::new() } else { spelled.get(vi).cloned().unwrap_or_default() };
                (s, format!("{}  {}", cb::voice_name(vi, n).to_uppercase(), note).trim().to_string())
            })
            .collect();
        // (the labels' room the same from chord to chord, whatever the notes, so the neck doesn't shift)
        let label_w = crate::font::keep_caps(|| {
            let mut most = 60.0_f32;
            for nv in 3..=6 {
                for vi in 0..nv {
                    let name = cb::voice_name(vi, nv).to_uppercase();
                    for note in ["F#", "Bb", "G#", "Eb"] {
                        most = most.max(crate::font::text_w(&format!("{name}  {note}"), 1.7));
                    }
                }
            }
            labels.iter().map(|(_, l)| crate::font::text_w(l, 1.7)).fold(most, f32::max)
        });
        let nut = r.min.x + label_w + 40.0;
        let room = r.max.x - nut - 10.0;
        // the frets shown: all of them when there's room, else the stretch around these chords, so the dots stay
        // big enough to read in a narrow window
        let frets: Vec<u8> = [to, from, ghosts.parent].iter().flatten().flat_map(|g| strings.iter().filter_map(|&s| g[s])).collect();
        let (first, count) = match (frets.iter().min(), frets.iter().max()) {
            (Some(&lo), Some(&hi)) if room / (FRETS as f32) < 30.0 => {
                let (lo, hi) = (lo as usize, hi as usize);
                let count = ((room / 30.0).floor() as usize).clamp(7, FRETS).max(hi - lo + 3).min(FRETS);
                // (it stays where it was while these chords fit in it, so it only moves when it has to)
                let (was, was_count) = NECK_VIEW.with(|v| v.get());
                let fits = was_count == count && (if lo == 0 { was == 0 } else { lo > was }) && hi <= was + count;
                let first = if fits {
                    was
                } else {
                    let first = lo.saturating_sub(1).min(FRETS - count);
                    if first <= 1 {
                        0
                    } else {
                        first
                    }
                };
                NECK_VIEW.with(|v| v.set((first, count)));
                (first, count)
            }
            _ => (0, FRETS),
        };
        let fw = room / count as f32;
        // (room above the top string for its dots and their rings, so they aren't cut off)
        let top = r.min.y + 18.0;
        let y = |s: usize| top + (5 - s) as f32 * gap;
        let x = |f: f32| if f <= 0.0 && first == 0 { nut - 16.0 } else { nut + (f - first as f32 - 0.5) * fw };
        // the board, frets and inlays
        p.rect_filled(Rect::from_min_max(Pos2::new(nut, top - 8.0), Pos2::new(r.max.x - 10.0, y(0) + 8.0)), 2.0, pal().display);
        for k in 0..=count {
            let fx = nut + k as f32 * fw;
            let is_nut = k == 0 && first == 0;
            let wire = pal().ink2.gamma_multiply(if is_nut { 0.9 } else { 0.4 });
            p.line_segment([Pos2::new(fx, top - 6.0), Pos2::new(fx, y(0) + 6.0)], Stroke::new(if is_nut { 4.0_f32 } else { 1.2 }, wire));
        }
        for k in [3usize, 5, 7, 9, 12, 15, 17, 19] {
            if k <= first || k > first + count {
                continue;
            }
            let cx = nut + ((k - first) as f32 - 0.5) * fw;
            let col = pal().ink2.gamma_multiply(0.35);
            if k == 12 {
                p.circle_filled(Pos2::new(cx, y(4) - gap * 0.5), 4.0, col);
                p.circle_filled(Pos2::new(cx, y(1) + gap * 0.5), 4.0, col);
            } else {
                p.circle_filled(Pos2::new(cx, (y(3) + y(2)) / 2.0), 4.0, col);
            }
            ptext(&p, Pos2::new(cx, y(0) + 22.0), Align::Center, &k.to_string(), 1.6, pal().ink2);
        }
        // up the neck: the first fret shown is numbered too
        if first > 0 && ![3usize, 5, 7, 9, 12, 15, 17, 19].contains(&(first + 1)) {
            ptext(&p, Pos2::new(nut + 0.5 * fw, y(0) + 22.0), Align::Center, &(first + 1).to_string(), 1.6, pal().ink2);
        }
        for s in 0..6 {
            let used = strings.iter().position(|x| *x == s);
            // (the strings with no notes thinner, in the same colour: on some looks the edge colour is the board's)
            p.line_segment([Pos2::new(nut, y(s)), Pos2::new(r.max.x - 10.0, y(s))], Stroke::new(if used.is_some() { 1.6_f32 } else { 1.0 }, pal().ink2));
            if let (Some(vi), Some((_, label))) = (used, labels.iter().find(|(x, _)| *x == s)) {
                crate::font::keep_caps(|| ptext(&p, Pos2::new(r.min.x + label_w + 4.0, y(s)), Align::Max, label, 1.7, voice_col(vi, n)));
            }
        }
        let Some(to) = to else {
            ptext(&p, Pos2::new(nut + 10.0, top + gap * 2.5), Align::Min, "These notes don't fit the strings in this order.", 2.0, crate::chords::CL_SEMI);
            return;
        };
        // the ghosts: the station it grew out of (faintest), then the chord it came from
        for (g, a) in [(ghosts.anchor, 0.16_f32), (ghosts.parent, 0.34)] {
            let Some(g) = g else { continue };
            for (vi, &s) in strings.iter().enumerate() {
                let Some(fr) = g[s] else { continue };
                let c = Pos2::new(x(fr as f32), y(s));
                p.circle_filled(c, 12.5, pal().ink.gamma_multiply(a * 0.6));
                p.circle_stroke(c, 12.5, Stroke::new(1.5_f32, voice_col(vi, n).gamma_multiply(a * 1.6)));
            }
        }
        // a thin line from where each moved voice was in the chord it came from
        if let Some(g) = ghosts.parent {
            for (vi, &s) in strings.iter().enumerate() {
                if let (Some(f0), Some(f1)) = (g[s], to[s]) {
                    if f0 != f1 && moved.contains(&vi) {
                        let (a, b) = (Pos2::new(x(f0 as f32), y(s)), Pos2::new(x(f1 as f32), y(s)));
                        let dir = if b.x > a.x { 1.0 } else { -1.0 };
                        p.line_segment([a + Vec2::new(13.0 * dir, 0.0), b - Vec2::new(13.0 * dir, 0.0)], Stroke::new(2.0_f32, voice_col(vi, n).gamma_multiply(0.55)));
                    }
                }
            }
        }
        for (vi, &s) in strings.iter().enumerate() {
            let Some(f1) = to[s] else { continue };
            let f0 = from.and_then(|g| g[s]).unwrap_or(f1);
            let fx = f0 as f32 + (f1 as f32 - f0 as f32) * t;
            let c = Pos2::new(x(fx), y(s));
            let lit = moved.contains(&vi);
            let col = if lit { voice_col(vi, n) } else { Color32::from_gray(235) };
            // a ghost where a moving voice was
            if lit && f0 != f1 && t < 1.0 {
                p.circle_stroke(Pos2::new(x(f0 as f32), y(s)), 12.0, Stroke::new(1.5_f32, voice_col(vi, n).gamma_multiply(1.0 - t)));
            }
            p.circle_filled(c, 12.5, col);
            if hidden {
                continue;
            }
            let d = degrees.get(vi).cloned().unwrap_or_default();
            let px = if d.chars().count() > 2 { 1.25 } else { 1.5 };
            crate::font::keep_caps(|| ptext(&p, c, Align::Center, &d, px, Color32::from_gray(15)));
        }
    }

    /// VOICE LINES: each voice's pitch through his order, a few voicings either side of this one (a column each),
    /// joined into lines; the voices that moved into each one marked; the stations numbered underneath, and an arrow
    /// from the column this one came from (`parent`; `anchor` is the station it grew out of). Returns a column clicked.
    fn cb_lines(ui: &mut egui::Ui, f: &Fam, i: usize, nv: usize, parent: Option<usize>, anchor: usize) -> Option<usize> {
        let n = f.walk.steps.len();
        let lo = i.saturating_sub(10);
        let hi = (lo + 21).min(n);
        let w = ui.available_width();
        let (r, resp) = ui.allocate_exact_size(Vec2::new(w, 176.0), Sense::click());
        // (kept inside its own box, whatever the window's width)
        let p = ui.painter_at(r);
        p.rect_filled(r, 2.0, pal().display);
        let pitches: Vec<Option<Vec<i32>>> = (lo..hi).map(|k| f.walk.steps[k].grip.map(|g| grip_pitches(&g))).collect();
        let all: Vec<i32> = pitches.iter().flatten().flatten().copied().collect();
        let (pmin, pmax) = (all.iter().min().copied().unwrap_or(40), all.iter().max().copied().unwrap_or(80));
        let cols = (hi - lo).max(1);
        let cw = (r.width() - 20.0) / cols as f32;
        let cx = |k: usize| r.min.x + 10.0 + (k - lo) as f32 * cw + cw / 2.0;
        // (room for the arrow above and the station numbers below)
        let (ptop, pbot) = (r.min.y + 26.0, r.max.y - 24.0);
        let py = |m: i32| pbot - (m - pmin) as f32 / (pmax - pmin).max(1) as f32 * (pbot - ptop);
        let col_rect = |k: usize| Rect::from_min_max(Pos2::new(cx(k) - cw / 2.0 + 1.0, ptop - 8.0), Pos2::new(cx(k) + cw / 2.0 - 1.0, pbot + 8.0));
        // this one's column, the station it grew out of (faint) and the one it came from (outlined)
        p.rect_filled(col_rect(i), 2.0, pal().sel);
        if anchor != i && (lo..hi).contains(&anchor) && Some(anchor) != parent {
            p.rect_filled(col_rect(anchor), 2.0, pal().sel.gamma_multiply(0.35));
        }
        if let Some(j) = parent {
            let y0 = r.min.y + 12.0;
            let to = Pos2::new(cx(i), y0);
            let from = if j >= lo {
                p.rect_stroke(col_rect(j), 2.0, Stroke::new(1.5_f32, pal().accent.gamma_multiply(0.8)));
                Pos2::new(cx(j), y0)
            } else {
                // further back than the graph shows
                ptext(&p, Pos2::new(r.min.x + 6.0, y0), Align::Min, &format!("FROM {}", j + 1), 1.4, pal().accent);
                Pos2::new(r.min.x + 60.0, y0)
            };
            let a = pal().accent;
            p.line_segment([from, to], Stroke::new(1.5_f32, a));
            p.line_segment([from, from + Vec2::new(0.0, 8.0)], Stroke::new(1.5_f32, a));
            p.line_segment([to, to + Vec2::new(-4.0, 6.0)], Stroke::new(1.5_f32, a));
            p.line_segment([to, to + Vec2::new(4.0, 6.0)], Stroke::new(1.5_f32, a));
        }
        // the stations (or parts) numbered under their columns
        for k in lo..hi {
            let t = match (f.walk.station_at(k), f.walk.stops.iter().position(|r| r.start == k)) {
                (Some(m), _) => format!("S{}", m + 1),
                (None, Some(n)) if !f.walk.inversions => format!("P{}", n + 1),
                _ => continue,
            };
            crate::font::keep_caps(|| ptext(&p, Pos2::new(cx(k), r.max.y - 10.0), Align::Center, &t, 1.4, pal().ink2));
        }
        for v in 0..nv {
            let col = voice_col(v, nv);
            let mut last: Option<Pos2> = None;
            for (k, ps) in (lo..hi).zip(&pitches) {
                let Some(m) = ps.as_ref().and_then(|ps| ps.get(v)) else {
                    last = None;
                    continue;
                };
                let pt = Pos2::new(cx(k), py(*m));
                if let Some(a) = last {
                    p.line_segment([a, pt], Stroke::new(1.5_f32, col.gamma_multiply(0.7)));
                }
                let moved = f.walk.steps[k].moved.contains(&v);
                p.circle_filled(pt, if moved { 5.0 } else { 3.0 }, col);
                last = Some(pt);
            }
        }
        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand).tip(
            "Each voice's line through the order (bass blue, tenor green, alto orange, top red; big dots moved). The arrow comes from the chord this one moved from; S1-S4 are the stations. Click a column to go there.",
        );
        if resp.clicked() {
            if let Some(pos) = resp.interact_pointer_pos() {
                let k = lo + ((pos.x - r.min.x - 10.0) / cw).floor().max(0.0) as usize;
                if k < hi {
                    return Some(k);
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn hidden_moves_keep_the_voice_and_the_step() {
        assert_eq!(super::hide_notes("Top voice B to C# (up a whole step)."), "Top voice up a whole step.");
        assert_eq!(
            super::hide_notes("Voice exchange: the bass and alto trade notes (G and F#)."),
            "Voice exchange: the bass and alto trade notes."
        );
        assert_eq!(
            super::hide_notes("Every voice moves up to the next note of the chord: Bass G to B (up a major 3rd); Top voice B to D (up a minor 3rd)."),
            "Every voice moves up to the next note of the chord: Bass up a major 3rd; Top voice up a minor 3rd."
        );
        assert_eq!(super::hide_notes("Alto Bb to Ab; Tenor C to D (up a whole step)."), "Alto; Tenor up a whole step.");
    }
}
