//! The LINES tab ATONAL, after Dennis Sandole. YOUR LINES: lines copied by hand from your own lesson pages, kept in
//! a file on this computer only (atonal_lines.json in Backline's folder; it is never part of Backline). Notes the
//! copy could not be sure of are marked; click one to fix it (on a small neck or a half step at a time), or delete
//! or add notes, and the file is saved. GENERATED LINES: made fresh from general rules - alternate triads from a
//! scale, two tetrachords joined into a synthetic scale, interval cycles. Under every line, what it is made of: the
//! interval between each pair of notes, its three- and four-note cells, the scales that hold all its notes, and its
//! syllables (moveable Do from the first note, and fixed Do).

use super::*;
use crate::ear;
use crate::lines;
use crate::theory;
use serde::{Deserialize, Serialize};
pub(crate) use crate::atonal_core::*;

// ------------------------------------------------------------------ the private file

/// One note of a copied line. `midi` is the note as written on the page in treble clef (middle C = 60; a guitar
/// sounds an octave lower), `len` its length in beats (0.25 a 16th), `unsure` set when the copy could not be sure of
/// it (fixing the note takes the mark off).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct AtNote {
    pub midi: i32,
    #[serde(default = "sixteenth")]
    pub len: f32,
    #[serde(default, skip_serializing_if = "is_false")]
    pub unsure: bool,
    /// sounds with the note before it (a chord on the page; kept, though the line plays its notes one by one)
    #[serde(default, skip_serializing_if = "is_false")]
    pub chord: bool,
}

fn sixteenth() -> f32 {
    0.25
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// A copied line: its name, the page's heading (the scale or chords it is built on, as written), where it is from,
/// what of it could not be read (said, never guessed), and its notes.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub(crate) struct AtLine {
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub header: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub gaps: String,
    #[serde(default)]
    pub notes: Vec<AtNote>,
}

/// The whole file:
/// `{"format": 1, "lines": [{"name": "Alternate triads p55, system 1", "header": "C-5 | A+4-9",
///   "source": "Sandole lesson page 55", "gaps": "...", "notes": [{"midi": 61, "len": 0.25, "unsure": true}, ...]}]}`
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub(crate) struct AtFile {
    #[serde(default = "format_one")]
    pub format: u32,
    #[serde(default)]
    pub lines: Vec<AtLine>,
}

fn format_one() -> u32 {
    1
}

/// Where the file lives: Backline's own folder on this computer.
pub(crate) fn file_path() -> std::path::PathBuf {
    crate::api::config_dir().join("atonal_lines.json")
}

/// The file read from its text (None when it isn't one).
pub(crate) fn parse(text: &str) -> Option<AtFile> {
    serde_json::from_str::<AtFile>(text).ok()
}

/// The file as text, one field to a line so it reads (and mends) by hand.
pub(crate) fn to_text(f: &AtFile) -> String {
    serde_json::to_string_pretty(f).unwrap_or_default()
}

fn load() -> Option<AtFile> {
    parse(&std::fs::read_to_string(file_path()).ok()?)
}

fn save(f: &AtFile) -> std::io::Result<()> {
    std::fs::write(file_path(), to_text(f))
}

/// Save one line: the file is read again first and only that line (by its name) is replaced, so lines added to the
/// file meanwhile (by another copy, by hand) are kept. The file as it now is.
fn save_line(line: &AtLine) -> std::io::Result<AtFile> {
    let mut f = load().unwrap_or(AtFile { format: 1, lines: Vec::new() });
    match f.lines.iter_mut().find(|l| l.name == line.name) {
        Some(l) => *l = line.clone(),
        None => f.lines.push(line.clone()),
    }
    save(&f)?;
    Ok(f)
}

/// When the private file was last written (to read it again when it changes).
fn file_time() -> Option<std::time::SystemTime> {
    std::fs::metadata(file_path()).and_then(|m| m.modified()).ok()
}

// ------------------------------------------------------------------ generated lines

/// The ways a line is generated.
pub(crate) const KINDS: [&str; 3] = ["ALTERNATE TRIADS", "TETRACHORDS JOINED", "INTERVAL CYCLE"];
/// The interval cycles: name, semitones.
pub(crate) const CYCLES: [(&str, i32); 4] = [("MINOR 3RDS", 3), ("MAJOR 3RDS", 4), ("4THS", 5), ("TRITONES", 6)];
/// Every scale the app knows for these lines: the scales and modes, then the synthetic and exotic ones (each set of
/// notes once), as (name, semitones over the root).
pub(crate) fn scale_list() -> Vec<(&'static str, Vec<i32>)> {
    let mut out: Vec<(&'static str, Vec<i32>)> = ear::SCALES.iter().map(|(n, s)| (*n, s.to_vec())).collect();
    for e in ear::EXOTICS.iter() {
        let mut st = ear::exotic_steps(e);
        st.sort();
        st.dedup();
        if !out.iter().any(|o| o.1 == st) {
            out.push((e.name, st));
        }
    }
    out
}

/// TETRACHORDS JOINED: the synthetic scale of one tetrachord on the root and another on the 5th (indexes of
/// ear::TETRACHORDS), as semitones over the root, low to high.
pub(crate) fn joined_scale(lower: usize, upper: usize) -> Vec<i32> {
    let lo = ear::TETRACHORDS[lower % ear::TETRACHORDS.len()].1;
    let up = ear::TETRACHORDS[upper % ear::TETRACHORDS.len()].1;
    let mut st: Vec<i32> = lo.iter().copied().chain(up.iter().map(|x| (x + 7) % 12)).collect();
    st.sort();
    st.dedup();
    st
}

/// The joined scale played up in four-note groups from each of its notes in turn, an octave of them.
pub(crate) fn tetrachord_line(root: i32, lower: usize, upper: usize) -> Vec<i32> {
    let st = joined_scale(lower, upper);
    let n = st.len();
    let base = base_of(root);
    let deg = |i: usize| base + st[i % n] + 12 * (i / n) as i32;
    (0..n).flat_map(|k| (k..k + 4).map(deg).collect::<Vec<_>>()).collect()
}

/// An interval in semitones as a short name, signed by direction: "+m3", "-P4", "+M9", "-P5+8" (a 5th and an
/// octave); "0" for a repeat.
pub(crate) fn interval_name(semis: i32) -> String {
    const NAMES: [&str; 12] = ["P8", "m2", "M2", "m3", "M3", "P4", "TT", "P5", "m6", "M6", "m7", "M7"];
    let a = semis.abs();
    if a == 0 {
        return "0".to_string();
    }
    let name = match ear::FREE.iter().find(|f| f.2 == a) {
        Some(f) if a != 6 => f.0.to_string(),
        _ if a < 12 => NAMES[a as usize].to_string(),
        // past a 10th: the simple interval and how many octaves over it
        _ => format!("{}+8", NAMES[(a % 12) as usize]),
    };
    format!("{}{}", if semis > 0 { "+" } else { "-" }, name)
}

/// The three-note cells a line is made of: the prime form of every three notes running (with three different pitch
/// classes), each once with how often it comes, the most often first.
pub(crate) fn trichords(line: &[i32]) -> Vec<(Vec<i32>, usize)> {
    let mut out: Vec<(Vec<i32>, usize)> = Vec::new();
    for w in line.windows(3) {
        let mut pcs: Vec<i32> = w.iter().map(|m| m.rem_euclid(12)).collect();
        pcs.sort();
        pcs.dedup();
        if pcs.len() < 3 {
            continue;
        }
        let p = ear::prime_form(&pcs);
        match out.iter_mut().find(|o| o.0 == p) {
            Some(o) => o.1 += 1,
            None => out.push((p, 1)),
        }
    }
    out.sort_by(|a, b| b.1.cmp(&a.1));
    out
}

/// The tetrachord (an index of ear::TETRACHORDS) four pitch classes make, read up from any of them.
pub(crate) fn tetrachord_of(pcs: &[i32]) -> Option<usize> {
    let mut v: Vec<i32> = pcs.iter().map(|m| m.rem_euclid(12)).collect();
    v.sort();
    v.dedup();
    if v.len() != 4 {
        return None;
    }
    (0..4).find_map(|k| {
        let mut rel: Vec<i32> = v.iter().map(|x| (x - v[k]).rem_euclid(12)).collect();
        rel.sort();
        ear::TETRACHORDS.iter().position(|t| t.1.as_slice() == rel.as_slice())
    })
}

/// The named tetrachords among every four notes running, each once with how often, the most often first.
pub(crate) fn tetrachords(line: &[i32]) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    for w in line.windows(4) {
        if let Some(t) = tetrachord_of(w) {
            match out.iter_mut().find(|o| o.0 == t) {
                Some(o) => o.1 += 1,
                None => out.push((t, 1)),
            }
        }
    }
    out.sort_by(|a, b| b.1.cmp(&a.1));
    out
}

/// The app's scales (scale_list) that hold every note of the line, in every key: (key, scale's index), the
/// synthetic and exotic ones first.
pub(crate) fn scales_holding(line: &[i32]) -> Vec<(i32, usize)> {
    let mut pcs: Vec<i32> = line.iter().map(|m| m.rem_euclid(12)).collect();
    pcs.sort();
    pcs.dedup();
    let list = scale_list();
    let exotic_from = ear::SCALES.len();
    let mut out: Vec<(i32, usize)> = Vec::new();
    for (i, (_, st)) in list.iter().enumerate() {
        for key in 0..12 {
            if pcs.iter().all(|p| st.contains(&(p - key).rem_euclid(12))) {
                out.push((key, i));
            }
        }
    }
    out.sort_by_key(|(k, i)| (*i < exotic_from, *i, *k));
    out
}

/// The line's syllables: moveable Do (its first note is Do) and fixed Do (C is Do), each note spelt the way it moves.
pub(crate) fn syllables(line: &[i32]) -> (Vec<&'static str>, Vec<&'static str>) {
    let moveable = ear::line_words(line, 0).into_iter().map(|w| w.1).collect();
    let fixed = ear::line_words(line, 1).into_iter().map(|w| w.1).collect();
    (moveable, fixed)
}

/// A written note's name with its octave: "D\u{266D}5".
fn note_label(m: i32) -> String {
    let (letter, acc, oct) = lines::spell(m, false);
    format!(
        "{}{}{}",
        ["C", "D", "E", "F", "G", "A", "B"][letter as usize],
        match acc {
            1 => "#",
            -1 => "\u{266D}",
            _ => "",
        },
        oct
    )
}

// ------------------------------------------------------------------ every string, every finger


// ------------------------------------------------------------------ the tab

/// Where the ATONAL tab is: which lines, the line open, the note being fixed, and the generator's choices.
pub(crate) struct AtState {
    /// 0 YOUR LINES, 1 GENERATED LINES
    pub view: u8,
    pub line: usize,
    pub sel: Option<usize>,
    /// the private file, read again whenever it changes on disk (new lines show up while the tab is open)
    pub file: Option<AtFile>,
    pub seen: Option<Option<std::time::SystemTime>>,
    /// what went wrong saving, if anything
    pub msg: String,
    pub kind: u8,
    pub key: i32,
    pub scale: usize,
    pub lower: usize,
    pub upper: usize,
    pub cycle: u8,
    /// EVERY STRING, EVERY FINGER: which starting place (by its order on the neck) and which finger
    pub start: usize,
    pub finger: u8,
}

impl Default for AtState {
    fn default() -> AtState {
        AtState {
            view: 0,
            line: 0,
            sel: None,
            file: None,
            seen: None,
            msg: String::new(),
            kind: 0,
            key: 0,
            // the first of the synthetic and exotic scales (the double harmonic)
            scale: ear::SCALES.len(),
            lower: 0,
            upper: 3,
            cycle: 0,
            start: 0,
            finger: 1,
        }
    }
}

/// One fact under a line: its name, then the text wrapped beside it.
fn fact(ui: &mut egui::Ui, name: &str, text: &str, col: Color32) {
    let lh = (crate::font::text_px(2.0) * 1.35).max(18.0);
    // (every fact's text starts in one column, past the longest name)
    let left = ["TRICHORDS", "MOVEABLE DO", "TETRACHORDS"].iter().map(|n| text_w(n, 2.0)).fold(0.0, f32::max) + 20.0;
    let w = (ui.available_width() - left - 12.0).max(120.0);
    for (k, line) in crate::font::wrap(text, 2.0, w).iter().enumerate() {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), lh), Sense::hover());
        if k == 0 {
            ptext(ui.painter(), Pos2::new(rect.min.x + 6.0, rect.center().y), Align::Min, name, 2.0, pal().ink2);
        }
        ptext(ui.painter(), Pos2::new(rect.min.x + left, rect.center().y), Align::Min, line, 2.0, col);
        let gap = ui.spacing().item_spacing.y;
        if gap > 0.0 {
            ui.add_space(-gap);
        }
    }
    ui.add_space(4.0);
}

impl App {
    pub(crate) fn atonal(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        para(ui, "Lines that belong to no key, the way Dennis Sandole taught them. YOUR LINES are the ones copied from your own lesson pages; GENERATED LINES are made fresh from his rules: alternate triads from a scale, two tetrachords joined into a synthetic scale, interval cycles. Under every line: its intervals, the cells it is made of, the scales that hold it and its syllables.", pal().ink2);
        if let Some(i) =
            crate::choice_line(ui, "at_view", "LINES", &["YOUR LINES", "GENERATED LINES"], self.at.view as usize, 220.0)
        {
            if i as u8 != self.at.view {
                self.at.view = i as u8;
                self.at.sel = None;
            }
        }
        if self.at.view == 0 {
            self.atonal_mine(ui, acts);
        } else {
            self.atonal_made(ui, acts);
        }
    }

    /// YOUR LINES: the lines in the private file, each played and written out, the unsure notes marked; a note
    /// clicked can be fixed, deleted or have one added after it, and the file is saved.
    fn atonal_mine(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let now = file_time();
        if self.at.seen != Some(now) {
            // (the open line stays open by its name)
            let open = self.at.file.as_ref().and_then(|f| f.lines.get(self.at.line)).map(|l| l.name.clone());
            self.at.file = load();
            self.at.seen = Some(now);
            if let (Some(name), Some(f)) = (open, self.at.file.as_ref()) {
                if let Some(k) = f.lines.iter().position(|l| l.name == name) {
                    self.at.line = k;
                }
            }
        }
        // a file of lines dropped on the window is taken in (copied into Backline's folder)
        let dropped: Vec<std::path::PathBuf> =
            ui.ctx().input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        if let Some(f) = dropped
            .iter()
            .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("json")))
            .find_map(|p| std::fs::read_to_string(p).ok().and_then(|t| parse(&t)))
        {
            self.at.msg = match save(&f) {
                Ok(()) => String::new(),
                Err(e) => format!("Couldn't keep the lines: {}", e),
            };
            self.at.file = Some(f);
            self.at.line = 0;
            self.at.sel = None;
        }
        let empty = self.at.file.as_ref().map_or(true, |f| f.lines.is_empty());
        if empty {
            para(ui, "No lines of your own yet. They are kept in a file on this computer only, atonal_lines.json in Backline's folder, and are never part of Backline. Drop a file of lines on the window to add it.", pal().ink2);
            crate::views::dim_line(ui, &file_path().display().to_string(), 1.0, pal().dim);
            return;
        }
        let Some(file) = self.at.file.as_ref() else { return };
        let n_lines = file.lines.len();
        self.at.line = self.at.line.min(n_lines - 1);
        let names: Vec<String> = file
            .lines
            .iter()
            .map(|l| {
                let u = l.notes.iter().filter(|n| n.unsure).count();
                if u > 0 {
                    format!("{}  ({} UNSURE)", l.name.to_uppercase(), u)
                } else {
                    l.name.to_uppercase()
                }
            })
            .collect();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        if let Some(i) = crate::choice_line(ui, "at_line", "LINE", &names, self.at.line, 320.0) {
            if i != self.at.line {
                self.at.line = i;
                self.at.sel = None;
            }
        }
        let line = file.lines[self.at.line].clone();
        let unsure = line.notes.iter().filter(|n| n.unsure).count();
        let mut about: Vec<String> = Vec::new();
        if !line.header.is_empty() {
            about.push(line.header.clone());
        }
        if !line.source.is_empty() {
            about.push(line.source.clone());
        }
        about.push(format!("{} NOTES", line.notes.len()));
        if unsure > 0 {
            about.push(format!("{} UNSURE - CLICK ONE TO FIX IT", unsure));
        }
        crate::views::dim_line(ui, &about.join("  -  "), 1.0, pal().ink2);
        if !line.gaps.is_empty() {
            crate::views::dim_line(ui, &format!("NOT READ: {}", line.gaps), 1.0, pal().dim);
        }
        if !self.at.msg.is_empty() {
            para(ui, &self.at.msg.clone(), pal().red);
        }
        let written: Vec<i32> = line.notes.iter().map(|n| n.midi).collect();
        if written.is_empty() {
            para(ui, "This line has no notes.", pal().ink2);
            return;
        }
        // played and drawn as a guitar sounds it: an octave under the page
        let sounding: Vec<i32> = written.iter().map(|m| m - 12).collect();
        let key = format!("atonal:mine:{}", line.name);
        self.line_player(ui, acts, &sounding, &key);
        let lit = self.seq_now("line");
        let warn = crate::ear::acc_col();
        let marks: Vec<Option<Color32>> = line.notes.iter().map(|n| n.unsure.then_some(warn)).collect();
        if let Some(sel) = self.at.sel {
            if sel >= written.len() {
                self.at.sel = None;
            }
        }
        let clicked =
            crate::lines_ui::line_score_marked(ui, acts, &sounding, self.ln_sub as usize, false, lit, None, &marks, self.at.sel);
        if let Some(i) = clicked {
            self.at.sel = if self.at.sel == Some(i) { None } else { Some(i) };
        }
        if let Some(i) = self.at.sel {
            self.atonal_fix(ui, acts, i, &sounding);
        }
        self.atonal_neck(ui, &sounding, lit);
        self.atonal_analysis(ui, &written);
    }

    /// Fixing note `i` of the open line: a half step down or up, or a place on the neck; marking it right as it is;
    /// deleting it, or adding a note after it. Every change is saved.
    fn atonal_fix(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, i: usize, sounding: &[i32]) {
        let li = self.at.line;
        let Some(note) = self.at.file.as_ref().and_then(|f| f.lines.get(li)).and_then(|l| l.notes.get(i)).cloned() else {
            return;
        };
        let n = sounding.len();
        // (what to do: a new note, a mark taken off, a delete, an add)
        let mut set: Option<i32> = None;
        let mut sure = false;
        let mut delete = false;
        let mut add = false;
        ui.add_space(4.0);
        section_header(ui, "FIX A NOTE");
        ui.horizontal_wrapped(|ui| {
            lcd_box(ui, &format!("NOTE {} OF {}", i + 1, n), 110.0, pal().ink);
            lcd_box(ui, &note_label(note.midi), 60.0, if note.unsure { crate::ear::acc_col() } else { pal().ink });
            ui.add_space(6.0);
            if retro_btn_w(ui, "-", 28.0, false).tip("A half step down.").clicked() {
                set = Some(note.midi - 1);
            }
            if retro_btn_w(ui, "+", 28.0, false).tip("A half step up.").clicked() {
                set = Some(note.midi + 1);
            }
            ui.add_space(6.0);
            if note.unsure
                && retro_btn(ui, "IT'S RIGHT", false).tip("Keep the note as it is and take off its unsure mark.").clicked()
            {
                sure = true;
            }
            if retro_btn(ui, "DELETE", false).tip("Take this note out of the line.").clicked() {
                delete = true;
            }
            if retro_btn(ui, "ADD A NOTE", false).tip("Put a copy of this note after it, to move where it belongs.").clicked() {
                add = true;
            }
            if retro_btn(ui, "DONE", false).clicked() {
                self.at.sel = None;
            }
        });
        crate::views::dim_line(ui, "Or click where the note is on the neck.", 1.0, pal().ink2);
        // the note where the line's fingering puts it; a click anywhere sets the note to that one
        let spots = lines::finger_line(sounding, &theory::STANDARD);
        let mut shape: [Option<u8>; 6] = [None; 6];
        if let Some((s, f)) = spots.get(i).copied().flatten() {
            shape[s] = Some(f);
        }
        let w = ui.available_width().min(720.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 120.0), Sense::hover());
        let neck = rect.shrink2(Vec2::new(0.0, 8.0));
        if let Some((s, f)) = crate::fretboard::neck(ui, neck, &shape, 15, true, "at_neck", None, &[], true) {
            match f {
                Some(f) => set = Some(theory::STANDARD[s] + f as i32 + 12),
                // the dot itself clicked: the note as it is
                None => sure = true,
            }
        }
        ui.add_space(14.0);
        if set.is_none() && !sure && !delete && !add {
            return;
        }
        let Some(file) = self.at.file.as_mut() else { return };
        let Some(line) = file.lines.get_mut(li) else { return };
        if let Some(m) = set {
            let m = m.clamp(36, 96);
            line.notes[i].midi = m;
            line.notes[i].unsure = false;
            acts.push(Action::PlayNotes(vec![m - 12]));
        }
        if sure {
            line.notes[i].unsure = false;
        }
        if add {
            let copy = AtNote { unsure: false, chord: false, ..line.notes[i].clone() };
            line.notes.insert(i + 1, copy);
            self.at.sel = Some(i + 1);
        }
        if delete {
            line.notes.remove(i);
            self.at.sel = if line.notes.is_empty() { None } else { Some(i.min(line.notes.len() - 1)) };
        }
        let edited = line.clone();
        match save_line(&edited) {
            Ok(fresh) => {
                self.at.line = fresh.lines.iter().position(|l| l.name == edited.name).unwrap_or(li);
                self.at.file = Some(fresh);
                self.at.seen = Some(file_time());
                self.at.msg = String::new();
            }
            Err(e) => self.at.msg = format!("Couldn't save the change: {}", e),
        }
    }

    /// GENERATED LINES: a line made from the rules, in a key, from a scale (or two tetrachords, or an interval).
    fn atonal_made(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        if let Some(i) = crate::choice_line(ui, "at_kind", "BUILT FROM", &KINDS, (self.at.kind as usize).min(2), 260.0) {
            self.at.kind = i as u8;
        }
        if let Some(pc) = Self::note_row(ui, "KEY", self.at.key, false) {
            self.at.key = pc;
        }
        let root = self.at.key;
        let (notes, about, key) = match self.at.kind {
            0 => {
                let list = scale_list();
                self.at.scale = self.at.scale.min(list.len() - 1);
                let names: Vec<&str> = list.iter().map(|s| s.0).collect();
                if let Some(i) = crate::choice_line(ui, "at_scale", "SCALE", &names, self.at.scale, 260.0) {
                    self.at.scale = i;
                }
                let st = &list[self.at.scale].1;
                let spelt: Vec<String> = st.iter().map(|s| theory::note_name(root + s, false).to_string()).collect();
                (
                    alternate_triads(root, st),
                    format!(
                        "{}: THE TRIAD ON EVERY OTHER NOTE OF {}, UP ONE, DOWN THE NEXT",
                        spelt.join(" ").to_uppercase(),
                        list[self.at.scale].0
                    ),
                    format!("atonal:triads:{}:{}", root, list[self.at.scale].0),
                )
            }
            1 => {
                let names: Vec<&str> = ear::TETRACHORDS.iter().map(|t| t.0).collect();
                if let Some(i) = crate::choice_line(ui, "at_lower", "LOWER", &names, self.at.lower, 200.0) {
                    self.at.lower = i;
                }
                if let Some(i) = crate::choice_line(ui, "at_upper", "UPPER", &names, self.at.upper, 200.0) {
                    self.at.upper = i;
                }
                let st = joined_scale(self.at.lower, self.at.upper);
                let spelt: Vec<String> = st.iter().map(|s| theory::note_name(root + s, false).to_string()).collect();
                (
                    tetrachord_line(root, self.at.lower, self.at.upper),
                    format!(
                        "{} ON THE ROOT, {} ON THE 5TH: {} - UP IN FOURS FROM EACH NOTE",
                        names[self.at.lower],
                        names[self.at.upper],
                        spelt.join(" ").to_uppercase()
                    ),
                    format!("atonal:tetra:{}:{}:{}", root, self.at.lower, self.at.upper),
                )
            }
            _ => {
                let names: Vec<&str> = CYCLES.iter().map(|c| c.0).collect();
                let ci = (self.at.cycle as usize).min(CYCLES.len() - 1);
                if let Some(i) = crate::choice_line(ui, "at_cycle", "INTERVAL", &names, ci, 200.0) {
                    self.at.cycle = i as u8;
                }
                let ci = (self.at.cycle as usize).min(CYCLES.len() - 1);
                (
                    interval_cycle(root, CYCLES[ci].1),
                    format!("ROUND THE CYCLE OF {} TILL IT COMES HOME, THEN AGAIN A HALF STEP UP, THE OTHER WAY", CYCLES[ci].0),
                    format!("atonal:cycle:{}:{}", root, ci),
                )
            }
        };
        crate::views::dim_line(ui, &about, 1.0, pal().ink2);
        if notes.is_empty() {
            return;
        }
        let sounding: Vec<i32> = notes.iter().map(|m| m - 12).collect();
        self.line_player(ui, acts, &sounding, &key);
        let lit = self.seq_now("line");
        crate::lines_ui::line_score(ui, acts, &sounding, self.ln_sub as usize, false, lit, None);
        self.atonal_neck(ui, &sounding, lit);
        self.atonal_analysis(ui, &notes);
    }

    /// EVERY STRING, EVERY FINGER under a line (sounding): pick where the first note goes and which finger plays it,
    /// and the whole line is shown on the neck in that position, each note with its finger.
    fn atonal_neck(&mut self, ui: &mut egui::Ui, sounding: &[i32], lit: Option<usize>) {
        let Some(first) = sounding.first() else { return };
        let all = starts(*first);
        if all.is_empty() {
            return;
        }
        ui.add_space(6.0);
        section_header(ui, "EVERY STRING, EVERY FINGER");
        crate::views::dim_line(ui, "FROM MEMORY: EACH START, EACH FINGER.", 1.0, pal().ink2);
        self.at.start = self.at.start.min(all.len() - 1);
        let names: Vec<String> = all.iter().map(|(s, f)| format!("{} STRING, FRET {}", STRING_NAMES[*s], f)).collect();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        if let Some(i) = crate::choice_line(ui, "at_start", "START", &names, self.at.start, 260.0) {
            self.at.start = i;
        }
        let start = all[self.at.start];
        let fingers: Vec<String> = (1..=4u8)
            .map(|k| match position_of(start, k) {
                Some(_) => format!("FINGER {}", k),
                None => format!("FINGER {} - OFF THE NECK", k),
            })
            .collect();
        let fingers: Vec<&str> = fingers.iter().map(String::as_str).collect();
        let fi = (self.at.finger.clamp(1, 4) - 1) as usize;
        if let Some(i) = crate::choice_line(ui, "at_finger", "FINGER", &fingers, fi, 260.0) {
            self.at.finger = i as u8 + 1;
        }
        let Some(pos) = position_of(start, self.at.finger.clamp(1, 4)) else {
            para(ui, "That finger can't reach this place: pick another finger or another place.", pal().ink2);
            return;
        };
        let (_, spots, moved) = finger_from(sounding, start, pos);
        let n_moved = moved.iter().filter(|m| **m).count();
        let mut about = format!("POSITION {}: FINGER 1 ON FRET {}", pos, pos);
        if n_moved > 0 {
            about.push_str(&format!("  -  {} NOTE{} AN OCTAVE OVER (RINGED)", n_moved, if n_moved == 1 { "" } else { "S" }));
        }
        crate::views::dim_line(ui, &about, 1.0, pal().ink2);
        let mut dots: Vec<crate::improv::NeckDot> = Vec::new();
        for (i, sp) in spots.iter().enumerate() {
            if dots.iter().any(|d| d.spot == *sp) {
                continue;
            }
            let k = finger_for(sp.1, pos);
            dots.push(crate::improv::NeckDot {
                spot: *sp,
                text: if k == 0 { "O".to_string() } else { k.to_string() },
                col: pal().ink,
                strength: 0.85,
                ring: moved[i],
                root: i == 0,
            });
        }
        crate::improv::neck_view(ui, &dots, lit.and_then(|i| spots.get(i).copied()));
    }

    /// ANALYSIS under a line (written notes): its intervals, cells, the scales that hold it, its syllables.
    fn atonal_analysis(&mut self, ui: &mut egui::Ui, line: &[i32]) {
        ui.add_space(6.0);
        section_header(ui, "ANALYSIS");
        let ink = pal().ink;
        let ints: Vec<String> = line.windows(2).map(|w| interval_name(w[1] - w[0])).collect();
        fact(ui, "INTERVALS", &ints.join("  "), ink);
        let tri: Vec<String> = trichords(line)
            .iter()
            .map(|(p, c)| {
                // written the usual short way, 10 and 11 as T and E: (014)
                let set: String =
                    p.iter().map(|x| ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'T', 'E'][*x as usize]).collect();
                if *c > 1 {
                    format!("({}) x{}", set, c)
                } else {
                    format!("({})", set)
                }
            })
            .collect();
        fact(ui, "TRICHORDS", &if tri.is_empty() { "NONE".to_string() } else { tri.join(",  ") }, ink);
        let tet: Vec<String> = tetrachords(line)
            .iter()
            .map(|(t, c)| if *c > 1 { format!("{} x{}", ear::TETRACHORDS[*t].0, c) } else { ear::TETRACHORDS[*t].0.to_string() })
            .collect();
        fact(ui, "TETRACHORDS", &if tet.is_empty() { "NONE NAMED".to_string() } else { tet.join(",  ") }, ink);
        let list = scale_list();
        let holding = scales_holding(line);
        let held: Vec<String> = holding
            .iter()
            .take(12)
            .map(|(k, i)| format!("{} {}", ear::KEYS[*k as usize].replace('\u{266D}', "b"), list[*i].0))
            .collect();
        let more = holding.len().saturating_sub(12);
        let held_text = if held.is_empty() {
            "None of the app's scales holds every note.".to_string()
        } else if more > 0 {
            format!("{},  AND {} MORE", held.join(",  "), more)
        } else {
            held.join(",  ")
        };
        fact(ui, "SCALES", &held_text, ink);
        let (moveable, fixed) = syllables(line);
        fact(ui, "MOVEABLE DO", &moveable.join(" "), ink);
        fact(ui, "FIXED DO", &fixed.join(" "), ink);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_triad(t: &[i32]) -> bool {
        let mut pcs: Vec<i32> = t.iter().map(|m| m.rem_euclid(12)).collect();
        pcs.sort();
        pcs.dedup();
        pcs.len() == 3
            && (0..3).any(|k| {
                let mut rel: Vec<i32> = pcs.iter().map(|x| (x - pcs[k]).rem_euclid(12)).collect();
                rel.sort();
                [[0, 4, 7], [0, 3, 7], [0, 3, 6], [0, 4, 8]].iter().any(|q| q.as_slice() == rel.as_slice())
            })
    }

    #[test]
    fn alternate_triads_are_the_scales_triads() {
        // C major: C E G up, then E G B down, G B D up ... every other degree, all seven
        let v = alternate_triads(0, &[0, 2, 4, 5, 7, 9, 11]);
        assert_eq!(v.len(), 21);
        assert_eq!(&v[..6], &[60, 64, 67, 71, 67, 64]);
        for t in v.chunks(3) {
            assert!(is_triad(t), "{:?}", t);
        }
        let roots: Vec<i32> = v.chunks(3).enumerate().map(|(k, t)| (if k % 2 == 0 { t[0] } else { t[2] }) % 12).collect();
        assert_eq!(roots, vec![0, 4, 7, 11, 2, 5, 9]);
        // every scale gives notes of its own, inside the range, in any key
        for (name, st) in scale_list() {
            for key in 0..12 {
                let v = alternate_triads(key, &st);
                assert_eq!(v.len(), 3 * st.len(), "{}", name);
                assert!(v.iter().all(|m| st.contains(&(m - key).rem_euclid(12))), "{}", name);
                assert!(v.iter().all(|m| (LOW..=HIGH + 12).contains(m)), "{} {:?}", name, v);
            }
        }
    }

    #[test]
    fn tetrachords_join_into_a_synthetic_scale() {
        // Lydian on C, Lydian on G: C D E F# G A B C#
        assert_eq!(joined_scale(2, 2), vec![0, 1, 2, 4, 6, 7, 9, 11]);
        // major + major is the major scale
        assert_eq!(joined_scale(0, 0), vec![0, 2, 4, 5, 7, 9, 11]);
        let v = tetrachord_line(0, 0, 0);
        assert_eq!(v.len(), 28);
        assert_eq!(&v[..8], &[60, 62, 64, 65, 62, 64, 65, 67]);
        assert!(v.windows(2).all(|w| w[0] != w[1]));
    }

    #[test]
    fn interval_cycles_go_round() {
        for (name, semis) in CYCLES {
            let v = interval_cycle(2, semis);
            let len = (12 / gcd(12, semis)) as usize;
            assert!(v.len() >= 24, "{}", name);
            for round in v.chunks(len) {
                // inside a round, every step is the interval (an octave either way), and it comes home after
                for w in round.windows(2) {
                    let d = (w[1] - w[0]).rem_euclid(12);
                    assert!(d == semis || d == 12 - semis, "{} {:?}", name, round);
                }
                let mut pcs: Vec<i32> = round.iter().map(|m| m % 12).collect();
                pcs.sort();
                pcs.dedup();
                assert_eq!(pcs.len(), len, "{}", name);
            }
            assert!(v.iter().all(|m| (LOW..=HIGH).contains(m)), "{} {:?}", name, v);
            // each round starts a half step higher
            assert_eq!((v[len] - v[0]).rem_euclid(12), 1);
        }
    }

    #[test]
    fn intervals_are_named_with_their_direction() {
        assert_eq!(interval_name(3), "+m3");
        assert_eq!(interval_name(-5), "-P4");
        assert_eq!(interval_name(6), "+TT");
        assert_eq!(interval_name(14), "+M9");
        assert_eq!(interval_name(-19), "-P5+8");
        assert_eq!(interval_name(0), "0");
    }

    #[test]
    fn cells_are_found() {
        // C Db E: 0 1 4; Db E G: 0 3 6; E G Bb: 0 3 6 again
        let t = trichords(&[60, 61, 64, 67, 70]);
        assert_eq!(t[0], (vec![0, 3, 6], 2));
        assert_eq!(t[1], (vec![0, 1, 4], 1));
        // a repeated note makes no cell
        assert!(trichords(&[60, 60, 62]).is_empty());
        // tetrachords, read up from any note: D E F G is the minor one, E F G A the Phrygian
        assert_eq!(tetrachord_of(&[62, 64, 65, 67]), Some(4));
        assert_eq!(tetrachord_of(&[69, 67, 65, 64]), Some(1));
        assert_eq!(tetrachord_of(&[60, 61, 64, 65]), Some(3));
        assert_eq!(tetrachord_of(&[60, 64, 67, 70]), None);
        let named = tetrachords(&[60, 62, 64, 65, 67, 69, 71, 72]);
        assert!(named.iter().any(|(t, _)| ear::TETRACHORDS[*t].0 == "MAJOR"));
    }

    #[test]
    fn scales_that_hold_a_line() {
        let list = scale_list();
        let name = |h: &(i32, usize)| (h.0, list[h.1].0);
        // C D# E G Ab B is the six-tone symmetrical scale on C (and on E and Ab)
        let held: Vec<(i32, &str)> = scales_holding(&[60, 63, 64, 67, 68, 71]).iter().map(name).collect();
        assert!(held.contains(&(0, "SIX-TONE SYMMETRICAL")));
        assert!(held.contains(&(4, "SIX-TONE SYMMETRICAL")));
        // the synthetic and exotic ones come first
        assert_eq!(held[0].1, "SIX-TONE SYMMETRICAL");
        // C Db E F G Ab B: double harmonic on C
        let held: Vec<(i32, &str)> = scales_holding(&[60, 61, 64, 65, 67, 68, 71]).iter().map(name).collect();
        assert!(held.contains(&(0, "DOUBLE HARMONIC")));
        // all twelve notes: no scale holds them
        assert!(scales_holding(&(60..72).collect::<Vec<_>>()).is_empty());
        // every scale is listed once
        for (i, a) in list.iter().enumerate() {
            assert!(!list[i + 1..].iter().any(|b| b.1 == a.1), "{} twice", a.0);
        }
    }

    #[test]
    fn syllables_from_the_first_note_and_from_c() {
        // D, up to F#, down to D\u{266D}: moveable (D is Do) Do Mi Ti; fixed Re Fi Ra
        let (m, f) = syllables(&[62, 66, 61]);
        assert_eq!(m, vec!["Do", "Mi", "Ti"]);
        assert_eq!(f, vec!["Re", "Fi", "Ra"]);
        // flats coming down: C Bb A
        let (m, f) = syllables(&[72, 70, 69]);
        assert_eq!(m, vec!["Do", "Te", "La"]);
        assert_eq!(f, vec!["Do", "Te", "La"]);
    }

    #[test]
    fn every_string_every_finger() {
        // middle C as written (C3 sounding): on the D string at fret 10, among others
        let all = starts(48);
        assert!(all.contains(&(2, 10)) && all.contains(&(3, 5)) && all.contains(&(4, 1)) && all.contains(&(1, 3)));
        assert!(all.iter().all(|(s, f)| (theory::STANDARD[*s] + *f as i32 - 48).rem_euclid(12) == 0));
        assert_eq!(position_of((2, 10), 1), Some(10));
        assert_eq!(position_of((2, 10), 4), Some(7));
        assert_eq!(position_of((4, 1), 2), None);
        // the alternate triads line from the D string, first finger: it starts there and stays near 10th position
        let line: Vec<i32> = vec![60, 63, 66, 62, 65, 69, 64, 67, 70, 73];
        let (played, spots, moved) = finger_from(&line, (2, 10), 10);
        assert_eq!(spots[0], (2, 10));
        assert_eq!(played, line);
        assert!(moved.iter().all(|m| !m));
        for (m, sp) in played.iter().zip(&spots) {
            assert_eq!(theory::STANDARD[sp.0] + sp.1 as i32, *m);
            assert!((9..=14).contains(&sp.1), "{:?}", spots);
        }
        // from the B string, fret 13: every note an octave higher
        let (played, spots, _) = finger_from(&line, (4, 13), 12);
        assert!(played.iter().zip(&line).all(|(a, b)| a - b == 12));
        assert_eq!(spots[0], (4, 13));
        assert_eq!(finger_for(10, 10), 1);
        assert_eq!(finger_for(13, 10), 4);
        assert_eq!(finger_for(14, 10), 4);
        assert_eq!(finger_for(0, 10), 0);
    }

    #[test]
    fn the_file_round_trips() {
        let f = AtFile {
            format: 1,
            lines: vec![AtLine {
                name: "Alternate triads, system 1".to_string(),
                header: "C-5 | A+4-9".to_string(),
                source: "a lesson page".to_string(),
                gaps: String::new(),
                notes: vec![AtNote { midi: 61, len: 0.25, unsure: true, chord: false }, AtNote { midi: 64, len: 0.25, unsure: false, chord: true }],
            }],
        };
        let text = to_text(&f);
        assert_eq!(parse(&text), Some(f.clone()));
        // a sure note and an empty field are left out of the file
        assert!(!text.contains("\"unsure\": false") && !text.contains("gaps"));
        // written by hand: the length and the format may be left out
        let hand = parse(r#"{"lines":[{"name":"x","notes":[{"midi":60},{"midi":62,"unsure":true}]}]}"#).unwrap();
        assert_eq!(hand.format, 1);
        assert_eq!(hand.lines[0].notes[0], AtNote { midi: 60, len: 0.25, unsure: false, chord: false });
        assert!(hand.lines[0].notes[1].unsure);
        assert!(parse("not a file").is_none());
    }

    /// The owner's own file, when it is on this computer: it reads, and every note is a sensible written pitch.
    #[test]
    #[ignore]
    fn the_private_file_reads() {
        let f = load().expect("no atonal_lines.json");
        assert!(!f.lines.is_empty());
        for l in &f.lines {
            assert!(l.notes.iter().all(|n| (40..=100).contains(&n.midi)), "{}", l.name);
        }
    }
}
