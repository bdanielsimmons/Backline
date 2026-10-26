//! DIARY > MY PLAN: a practice plan of your own, kept in a file on this computer only (plan.json in Backline's
//! folder; the tab shows only when the file is there, and it is never part of Backline). The plan has weeks (a
//! phase, a project with its finish line, a tune, a line, a rung of the sing-then-play ladder), the days of a week,
//! and the blocks of a day's session. Today's blocks run on a timer: start, pause, skip back and forward, a chime
//! when a block is up, and what was done is kept (plan_log.json).

use super::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

// ------------------------------------------------------------------ the private file

/// One block of a session: its name, how long, and what to do (`{tune}`, `{tune_step}`, `{line}`, `{rung}`,
/// `{rung_how}` and `{project}` are filled in from the week and the day).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct Block {
    pub name: String,
    pub min: u32,
    #[serde(default)]
    pub what: String,
    /// what to do when you are ahead of the plan, and when behind
    #[serde(default)]
    pub ahead: String,
    #[serde(default)]
    pub behind: String,
}

/// A tune's changes, a bar each (two chords in a bar with a space between).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub(crate) struct Changes {
    pub name: String,
    pub bars: Vec<String>,
}

/// A day of the week (Monday first): whether it runs the daily core, its own blocks (after the core, or alone),
/// a note, and that day's step with the tune of the week.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct Day {
    pub name: String,
    #[serde(default)]
    pub core: bool,
    #[serde(default)]
    pub blocks: Vec<Block>,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub tune_step: String,
}

/// A week: its phase, the project (and when it is done, and what to work on), the tune, the line, the rung.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct Week {
    #[serde(default)]
    pub phase: String,
    #[serde(default)]
    pub project: String,
    #[serde(default)]
    pub done: String,
    #[serde(default)]
    pub focus: Vec<String>,
    #[serde(default)]
    pub tune: String,
    #[serde(default)]
    pub line: String,
    /// which rung of the ladder (1 the first)
    #[serde(default)]
    pub rung: usize,
    /// the project's step for each day, Monday first
    #[serde(default)]
    pub steps: Vec<String>,
    /// the changes of the week's tunes, for the guide tones and arpeggios on the neck
    #[serde(default)]
    pub changes: Vec<Changes>,
    /// a song of the week for a gig of your own ({song})
    #[serde(default)]
    pub song: String,
}

/// A rung of the sing-then-play ladder: its name and exactly how to do it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct Rung {
    pub name: String,
    pub how: String,
}

/// A section of notes at the end (a title and its points).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct Notes {
    pub title: String,
    pub points: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct Plan {
    #[serde(default)]
    pub title: String,
    /// the Monday week 1 starts, YYYY-MM-DD
    pub start: String,
    #[serde(default)]
    pub blocks: Vec<Block>,
    pub days: Vec<Day>,
    pub weeks: Vec<Week>,
    #[serde(default)]
    pub ladder_about: String,
    #[serde(default)]
    pub rungs: Vec<Rung>,
    #[serde(default)]
    pub notes: Vec<Notes>,
}

impl Plan {
    /// This week's number (1 the first; less before the plan starts, more after it ends).
    pub(crate) fn start_week(&self) -> i64 {
        day_number(&self.start).map_or(0, |s| week_of(s, store::today_days()))
    }
}

pub(crate) fn file_path() -> std::path::PathBuf {
    crate::api::config_dir().join("plan.json")
}

fn log_path() -> std::path::PathBuf {
    crate::api::config_dir().join("plan_log.json")
}

pub(crate) fn parse(text: &str) -> Result<Plan, String> {
    let p: Plan = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if p.days.len() != 7 {
        return Err(format!("the plan needs 7 days, Monday first (it has {})", p.days.len()));
    }
    if p.weeks.is_empty() {
        return Err("the plan has no weeks".to_string());
    }
    if day_number(&p.start).is_none() {
        return Err(format!("the start date \"{}\" isn't a date like 2026-10-26", p.start));
    }
    Ok(p)
}

fn file_time() -> Option<std::time::SystemTime> {
    std::fs::metadata(file_path()).and_then(|m| m.modified()).ok()
}

/// Whether there is a plan file (the tab shows only then).
pub(crate) fn have_plan() -> bool {
    file_path().exists()
}

/// Days since 1970 for YYYY-MM-DD.
pub(crate) fn day_number(s: &str) -> Option<i64> {
    let mut it = s.trim().split('-');
    let y: i32 = it.next()?.parse().ok()?;
    let m: u32 = it.next()?.parse().ok()?;
    let d: u32 = it.next()?.parse().ok()?;
    if it.next().is_some() || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some(store::days_from_civil(y, m, d))
}

/// The day of the week for days since 1970, Monday 0.
pub(crate) fn weekday(days: i64) -> usize {
    // 1 January 1970 was a Thursday
    (days + 3).rem_euclid(7) as usize
}

/// Which week of the plan a day is in, 1 the first (0 before it starts; past the end, the number past the end).
pub(crate) fn week_of(start: i64, today: i64) -> i64 {
    if today < start {
        0
    } else {
        (today - start) / 7 + 1
    }
}

/// A block's text with the week's and day's things filled in.
/// How far you have moved ahead of (or behind) the plan, block by block: the ladder's rung, the project's step
/// (it carries on into the next week's steps) and the tune's step (this week only). Kept in plan_moves.json.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub(crate) struct Moves {
    #[serde(default)]
    pub rung: i32,
    #[serde(default)]
    pub project: i32,
    #[serde(default)]
    pub tune: i32,
    /// the week the tune's move is for (it starts again each week)
    #[serde(default)]
    pub tune_week: i64,
}

/// Something from the fun slot worth keeping (a lick, a phrase, a bit of a solo): the days you played it cold,
/// and the last day it came up in Review. Kept in plan_keepers.json.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub(crate) struct Keeper {
    pub text: String,
    #[serde(default)]
    pub added: String,
    /// the days it was played cold, from memory
    #[serde(default)]
    pub played: Vec<String>,
    /// the last day it came up (played or not)
    #[serde(default)]
    pub seen: String,
}

impl Keeper {
    /// Solid: played cold on 3 days or more, a week or more apart from first to last.
    pub(crate) fn solid(&self) -> bool {
        let days: Vec<i64> = self.played.iter().filter_map(|d| day_number(d)).collect();
        days.len() >= 3 && days.iter().max().unwrap_or(&0) - days.iter().min().unwrap_or(&0) >= 7
    }
}

/// Today's keepers for Review: the ones already up today, then the longest unseen (new ones first), two in all.
/// A solid one comes back only every two weeks.
pub(crate) fn keepers_today(ks: &[Keeper], today: &str, n: usize) -> Vec<usize> {
    let t = day_number(today).unwrap_or(0);
    let mut out: Vec<usize> = (0..ks.len()).filter(|i| ks[*i].seen == today).take(n).collect();
    let mut rest: Vec<usize> = (0..ks.len())
        .filter(|i| ks[*i].seen != today)
        .filter(|i| !ks[*i].solid() || day_number(&ks[*i].seen).map_or(true, |s| t - s >= 14))
        .collect();
    rest.sort_by_key(|i| (day_number(&ks[*i].seen).unwrap_or(i64::MIN), day_number(&ks[*i].added).unwrap_or(0)));
    out.extend(rest.into_iter().take(n.saturating_sub(out.len())));
    out
}

fn keepers_path() -> std::path::PathBuf {
    crate::api::config_dir().join("plan_keepers.json")
}

fn moves_path() -> std::path::PathBuf {
    crate::api::config_dir().join("plan_moves.json")
}

/// What a block moves along when you go ahead or back: the rung, the project's step or the tune's step (from what
/// its text fills in), or nothing.
pub(crate) fn block_moves(b: &Block) -> Option<&'static str> {
    if b.what.contains("{rung}") {
        Some("rung")
    } else if b.what.contains("{project_step}") {
        Some("project")
    } else if b.what.contains("{tune_step}") {
        Some("tune")
    } else {
        None
    }
}

/// The rung you are on: the week's, moved.
pub(crate) fn rung_now(plan: &Plan, week: &Week, mv: &Moves) -> usize {
    (week.rung.max(1) as i32 + mv.rung).clamp(1, plan.rungs.len().max(1) as i32) as usize
}

/// The project's step for this day: every week's steps in a row, moved along them. The week it comes from (1 the
/// first) and its place in that week, or None when there are no steps.
pub(crate) fn project_at(plan: &Plan, week: &Week, wd: usize, mv: &Moves) -> Option<(usize, usize)> {
    let wi = plan.weeks.iter().position(|w| w == week).unwrap_or(0);
    let all: Vec<(usize, usize)> = plan.weeks.iter().enumerate().flat_map(|(i, w)| (0..w.steps.len()).map(move |k| (i, k))).collect();
    if all.is_empty() {
        return None;
    }
    let here = plan.weeks[..wi].iter().map(|w| w.steps.len()).sum::<usize>() + wd.min(week.steps.len().saturating_sub(1));
    let g = (here as i32 + mv.project).clamp(0, all.len() as i32 - 1) as usize;
    let (i, k) = all[g];
    Some((i + 1, k))
}

/// The tune's step for this day, moved along the week's days; past the last one, on to the next week's tune.
pub(crate) fn tune_step_at(plan: &Plan, week: &Week, wd: usize, mv: &Moves, week_no: i64) -> String {
    let steps: Vec<&str> = plan.days.iter().map(|d| d.tune_step.as_str()).filter(|s| !s.is_empty()).collect();
    if steps.is_empty() {
        return String::new();
    }
    let off = if mv.tune_week == week_no { mv.tune } else { 0 };
    let k = (wd.min(steps.len() - 1) as i32 + off).max(0) as usize;
    if k < steps.len() {
        return steps[k].to_string();
    }
    let wi = plan.weeks.iter().position(|w| w == week).unwrap_or(0);
    match plan.weeks.get(wi + 1) {
        Some(next) => format!("this week's melody is done. Start next week's tune, {}: {}", next.tune, steps[0]),
        None => "every melody in the plan is done. Play them all from memory, and pick a new one.".to_string(),
    }
}

pub(crate) fn fill(text: &str, plan: &Plan, week: &Week, day: &Day) -> String {
    fill_moved(text, plan, week, day, &Moves::default(), 0)
}

/// A block's text filled in, with your moves ahead or behind in it.
pub(crate) fn fill_moved(text: &str, plan: &Plan, week: &Week, day: &Day, mv: &Moves, week_no: i64) -> String {
    let wd = plan.days.iter().position(|d| d.name == day.name).unwrap_or(0);
    let wi = plan.weeks.iter().position(|w| w == week).unwrap_or(0);
    let step = match project_at(plan, week, wd, mv) {
        Some((w, k)) if w == wi + 1 => plan.weeks[w - 1].steps[k].clone(),
        Some((w, k)) => format!("(from week {}) {}", w, plan.weeks[w - 1].steps[k]),
        None => String::new(),
    };
    let tune_step = if mv.tune == 0 || mv.tune_week != week_no { day.tune_step.clone() } else { tune_step_at(plan, week, wd, mv, week_no) };
    let rung = rung_now(plan, week, mv);
    let (rname, rhow) = plan
        .rungs
        .get(rung - 1)
        .map(|r| (format!("rung {}, {}", rung, r.name), r.how.clone()))
        .unwrap_or_else(|| (format!("rung {}", rung), String::new()));
    text.replace("{tune}", &week.tune)
        .replace("{tune_step}", &tune_step)
        .replace("{line}", &week.line)
        .replace("{rung}", &rname)
        .replace("{rung_how}", &rhow)
        .replace("{project_step}", &step)
        .replace("{project}", &week.project)
        .replace("{song}", &week.song)
}

/// A block's colour, by what it is (from its name), the same wherever it is drawn.
pub(crate) fn block_col(name: &str) -> Color32 {
    let n = name.to_lowercase();
    let pick = [
        ("warm", (185, 160, 230)),
        ("ear", (143, 179, 239)),
        ("sandole", (239, 143, 176)),
        ("tune", (111, 211, 211)),
        ("project", (111, 207, 143)),
        ("review", (163, 177, 198)),
        ("bonus", (240, 180, 107)),
        ("fun", (232, 210, 122)),
        ("church", (217, 179, 140)),
    ];
    let (r, g, b) = pick.iter().find(|(k, _)| n.contains(k)).map(|x| x.1).unwrap_or((150, 156, 168));
    Color32::from_rgb(r, g, b)
}

/// A phase's colour (by its place among the plan's phases).
fn phase_col(k: usize) -> Color32 {
    [Color32::from_rgb(111, 207, 143), Color32::from_rgb(143, 179, 239), Color32::from_rgb(239, 143, 176), Color32::from_rgb(240, 180, 107)][k % 4]
}

/// Text in a box only when it fits (else its first word when that fits, else nothing; the box's tip has it all).
fn label_in(p: &egui::Painter, at: Pos2, text: &str, px: f32, room: f32, col: Color32) {
    if text_w(text, px) <= room {
        ptext(p, at, Align::Center, text, px, col);
    } else if let Some(w) = text.split_whitespace().next().filter(|w| text_w(w, px) <= room) {
        ptext(p, at, Align::Center, w, px, col);
    }
}

/// The short name of a day (MON).
fn short_day(name: &str) -> String {
    name.chars().take(3).collect::<String>().to_uppercase()
}

/// Blocks drawn as one bar, each as wide as its minutes (out of `scale` minutes across the bar), in its colour and
/// named; `cur` boxed, the `done` ones ticked and dimmed. Returns the block clicked.
fn block_bar(ui: &mut egui::Ui, id: &str, blocks: &[Block], scale: u32, h: f32, cur: Option<usize>, done: &[u8]) -> Option<usize> {
    let w = ui.available_width().min(1000.0);
    let (bar, _) = ui.allocate_exact_size(Vec2::new(w, h), Sense::hover());
    let mut x = bar.min.x;
    let mut clicked = None;
    for (k, b) in blocks.iter().enumerate() {
        let bw = bar.width() * b.min as f32 / scale.max(1) as f32;
        let r = Rect::from_min_size(Pos2::new(x, bar.min.y), Vec2::new(bw, h)).shrink2(Vec2::new(1.0, 0.0));
        let resp = ui.interact(r, ui.id().with((id, k)), Sense::click());
        let is_done = done.contains(&(k as u8));
        let is_cur = cur == Some(k);
        let col = block_col(&b.name);
        let fill_c = if is_cur {
            col
        } else if is_done || cur.is_some() {
            col.gamma_multiply(0.45)
        } else {
            col.gamma_multiply(0.85)
        };
        ui.painter().rect_filled(r, 4.0, fill_c);
        if is_cur {
            ui.painter().rect_stroke(r.expand(1.5), 5.0, Stroke::new(2.5_f32, pal().ink));
        } else if resp.hovered() {
            ui.painter().rect_stroke(r, 4.0, Stroke::new(1.5_f32, pal().ink));
        }
        let ink = Color32::from_gray(14);
        let label = if is_done && !is_cur { format!("{}  OK", b.name) } else { b.name.clone() };
        if h >= 40.0 {
            label_in(ui.painter(), Pos2::new(r.center().x, r.center().y - 8.0), &label, 1.5, r.width() - 6.0, ink);
            label_in(ui.painter(), Pos2::new(r.center().x, r.center().y + 9.0), &format!("{} MIN", b.min), 1.2, r.width() - 6.0, ink);
        } else {
            label_in(ui.painter(), r.center(), &label, 1.4, r.width() - 6.0, ink);
        }
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).tip(format!("{}: {} min", b.name, b.min)).clicked() {
            clicked = Some(k);
        }
        x += bw;
    }
    clicked
}

/// The weeks as a strip of boxes coloured by phase (this week dotted, the one looked at boxed), the phases named
/// under it. Returns the week clicked (1 the first).
fn week_strip(ui: &mut egui::Ui, plan: &Plan, now_w: i64, shown: i64) -> Option<i64> {
    let n = plan.weeks.len();
    let mut phases: Vec<&str> = Vec::new();
    for w in &plan.weeks {
        if !phases.contains(&w.phase.as_str()) {
            phases.push(&w.phase);
        }
    }
    let w = ui.available_width().min(1000.0);
    let (bar, _) = ui.allocate_exact_size(Vec2::new(w, 70.0), Sense::hover());
    let bw = bar.width() / n as f32;
    let mut clicked = None;
    for (k, wk) in plan.weeks.iter().enumerate() {
        let pk = phases.iter().position(|p| *p == wk.phase).unwrap_or(0);
        let r = Rect::from_min_size(Pos2::new(bar.min.x + k as f32 * bw, bar.min.y), Vec2::new(bw, 44.0)).shrink2(Vec2::new(2.0, 0.0));
        let resp = ui.interact(r, ui.id().with(("plan_wk", k)), Sense::click());
        let num = k as i64 + 1;
        let col = phase_col(pk);
        let past = num < now_w;
        ui.painter().rect_filled(r, 5.0, if num == shown { col } else { col.gamma_multiply(if past { 0.3 } else { 0.55 }) });
        if num == shown {
            ui.painter().rect_stroke(r.expand(1.5), 6.0, Stroke::new(2.5_f32, pal().ink));
        } else if resp.hovered() {
            ui.painter().rect_stroke(r, 5.0, Stroke::new(1.5_f32, pal().ink));
        }
        let ink = Color32::from_gray(14);
        let label = if num == now_w { format!("W{} NOW", num) } else { format!("W{}", num) };
        label_in(ui.painter(), Pos2::new(r.center().x, r.center().y - 7.0), &label, 1.6, r.width() - 4.0, ink);
        label_in(ui.painter(), Pos2::new(r.center().x, r.center().y + 9.0), &wk.tune, 1.1, r.width() - 4.0, ink);
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).tip(format!("Week {}: {}", num, wk.project)).clicked() {
            clicked = Some(num);
        }
    }
    // the phases, each under its weeks
    let mut k = 0;
    while k < n {
        let ph = &plan.weeks[k].phase;
        let mut e = k;
        while e + 1 < n && plan.weeks[e + 1].phase == *ph {
            e += 1;
        }
        let x0 = bar.min.x + k as f32 * bw + 2.0;
        let x1 = bar.min.x + (e + 1) as f32 * bw - 2.0;
        let y = bar.min.y + 52.0;
        let pk = phases.iter().position(|p| *p == ph.as_str()).unwrap_or(0);
        ui.painter().line_segment([Pos2::new(x0, y - 4.0), Pos2::new(x1, y - 4.0)], Stroke::new(2.0_f32, phase_col(pk)));
        label_in(ui.painter(), Pos2::new((x0 + x1) / 2.0, y + 8.0), &ph.to_uppercase(), 1.2, x1 - x0, pal().ink2);
        k = e + 1;
    }
    clicked
}

/// The blocks of a day: the core (when the day runs it), then the day's own.
pub(crate) fn day_blocks(plan: &Plan, day: &Day) -> Vec<Block> {
    let mut v = if day.core { plan.blocks.clone() } else { Vec::new() };
    v.extend(day.blocks.iter().cloned());
    v
}

fn mmss(secs: u64) -> String {
    format!("{:02}:{:02}", secs / 60, secs % 60)
}

// ------------------------------------------------------------------ the tab's state

/// MY PLAN: the plan (read again when the file changes), the week looked at, and today's session timer.
pub(crate) struct PlanState {
    pub file: Option<Result<Plan, String>>,
    pub seen: Option<Option<std::time::SystemTime>>,
    /// the week looked at, as an offset from this week
    pub look: i64,
    /// the block of today's session, and the timer: running till `end`, or stopped with `left` seconds to go
    pub block: usize,
    pub end: Option<Instant>,
    pub left: u64,
    /// start the next block on its own when one is up
    pub auto: bool,
    /// the day the timer is for (it starts again on a new day)
    pub day: i64,
    /// blocks done, by date
    pub log: Option<BTreeMap<String, Vec<u8>>>,
    /// GUIDE TONES AND ARPEGGIOS: which (0 guide tones, 1 arpeggios), which tune of the week, which chord, where
    pub gt_kind: u8,
    pub gt_set: usize,
    pub gt_chord: usize,
    pub gt_area: u8,
    /// how far ahead or behind you have moved (plan_moves.json)
    pub moves: Option<Moves>,
    /// the keepers (plan_keepers.json), and the one being written
    pub keepers: Option<Vec<Keeper>>,
    pub keeper_new: String,
}

impl Default for PlanState {
    fn default() -> PlanState {
        PlanState { file: None, seen: None, look: 0, block: 0, end: None, left: 0, auto: true, day: -1, log: None, gt_kind: 0, gt_set: 0, gt_chord: 0, gt_area: 3, moves: None, keepers: None, keeper_new: String::new() }
    }
}

impl PlanState {
    fn load(&mut self) {
        let now = file_time();
        if self.seen != Some(now) {
            self.file = std::fs::read_to_string(file_path()).ok().map(|t| parse(&t));
            self.seen = Some(now);
        }
        if self.log.is_none() {
            self.log = Some(
                std::fs::read_to_string(log_path()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default(),
            );
        }
    }

    fn mv(&mut self) -> Moves {
        self.moves
            .get_or_insert_with(|| {
                std::fs::read_to_string(moves_path()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
            })
            .clone()
    }

    fn set_mv(&mut self, m: Moves) {
        if let Ok(t) = serde_json::to_string_pretty(&m) {
            let _ = std::fs::write(moves_path(), t);
        }
        self.moves = Some(m);
    }

    fn keepers(&mut self) -> &mut Vec<Keeper> {
        self.keepers.get_or_insert_with(|| {
            std::fs::read_to_string(keepers_path()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
        })
    }

    fn save_keepers(&mut self) {
        if let Ok(t) = serde_json::to_string_pretty(self.keepers()) {
            let _ = std::fs::write(keepers_path(), t);
        }
    }

    fn done_today(&self) -> Vec<u8> {
        self.log.as_ref().and_then(|l| l.get(&store::today_str())).cloned().unwrap_or_default()
    }

    /// Tick a block done today (or not); true when it was newly ticked.
    fn mark(&mut self, block: usize, on: bool) -> bool {
        let date = store::today_str();
        let log = self.log.get_or_insert_with(BTreeMap::new);
        let v = log.entry(date).or_default();
        let b = block as u8;
        let mut new = false;
        if on && !v.contains(&b) {
            v.push(b);
            v.sort();
            new = true;
        } else if !on {
            v.retain(|x| *x != b);
        }
        if let Ok(t) = serde_json::to_string_pretty(log) {
            let _ = std::fs::write(log_path(), t);
        }
        new
    }

    /// Seconds left on the current block.
    fn secs_left(&self) -> u64 {
        match self.end {
            Some(e) => e.saturating_duration_since(Instant::now()).as_secs() + u64::from(Instant::now() < e),
            None => self.left,
        }
    }

    /// Go to block `k` of a session of these blocks, stopped (or running when `run`).
    fn go(&mut self, blocks: &[Block], k: usize, run: bool) {
        self.block = k.min(blocks.len().saturating_sub(1));
        let secs = blocks.get(self.block).map_or(0, |b| b.min as u64 * 60);
        if run {
            self.end = Some(Instant::now() + Duration::from_secs(secs));
        } else {
            self.end = None;
            self.left = secs;
        }
    }

    /// Today's blocks (from today's day and this week), if there is a plan.
    fn today(&self) -> Option<(Plan, Week, Day, Vec<Block>)> {
        let plan = self.file.as_ref()?.as_ref().ok()?.clone();
        let start = day_number(&plan.start)?;
        let today = store::today_days();
        let w = week_of(start, today).clamp(1, plan.weeks.len() as i64);
        let week = plan.weeks[(w - 1) as usize].clone();
        let day = plan.days[weekday(today)].clone();
        let blocks = day_blocks(&plan, &day);
        Some((plan, week, day, blocks))
    }
}

/// Text wrapped to the width, always shown whole.
fn text_block(ui: &mut egui::Ui, text: &str, px: f32, col: Color32, indent: f32) {
    let lh = (crate::font::text_px(px) * 1.35).max(18.0);
    let w = (ui.available_width() - indent - 12.0).max(120.0);
    for line in crate::font::wrap(text, px, w) {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), lh), Sense::hover());
        ptext(ui.painter(), Pos2::new(rect.min.x + indent, rect.center().y), Align::Min, &line, px, col);
        let gap = ui.spacing().item_spacing.y;
        if gap > 0.0 {
            ui.add_space(-gap);
        }
    }
    ui.add_space(4.0);
}

/// One fact: its name, then its text wrapped beside it.
fn fact(ui: &mut egui::Ui, name: &str, text: &str, col: Color32) {
    if text.is_empty() {
        return;
    }
    let lh = (crate::font::text_px(2.0) * 1.35).max(18.0);
    let left = ["DONE WHEN", "PROJECT", "EAR LADDER"].iter().map(|n| text_w(n, 2.0)).fold(0.0, f32::max) + 20.0;
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

/// The neck's areas to look in: a name and the frets (the whole neck last).
const AREAS: [(&str, u8, u8); 6] =
    [("FRETS 0-4", 0, 4), ("3-7", 3, 7), ("5-9", 5, 9), ("7-11", 7, 11), ("10-14", 10, 14), ("WHOLE NECK", 0, 15)];

/// Each chord once, in the order it comes in the changes.
pub(crate) fn chords_once(bars: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for c in bars.iter().flat_map(|b| b.split_whitespace()) {
        if !out.iter().any(|o| o == c) {
            out.push(c.to_string());
        }
    }
    out
}

/// A chord's 3rd and 7th (guide tones) or its 1 3 5 7 (arpeggio), as intervals from the root (the 6th of a 6
/// chord or the bb7 of a dim7 stands in for the 7th). With its root, or None when the chord can't be read.
pub(crate) fn chord_parts(name: &str, arp: bool) -> Option<(i32, Vec<i32>)> {
    let n = crate::theory::chord_notes(name)?;
    let r = n.root.rem_euclid(12);
    let has = |iv: i32| n.pcs.iter().any(|p| (p - r).rem_euclid(12) == iv);
    let third = if has(4) { 4 } else if has(3) { 3 } else { 5 };
    let fifth = if has(7) { 7 } else if has(6) { 6 } else if has(8) { 8 } else { 7 };
    let seventh = [10, 11, 9].into_iter().find(|iv| has(*iv));
    let mut v = if arp { vec![0, third, fifth] } else { vec![third] };
    v.extend(seventh);
    Some((r, v))
}

/// The dots for a chord on the neck in these frets: its guide tones (with the root dim, to find them by) or its
/// arpeggio, each named by its degree and coloured by it.
fn chord_dots(name: &str, arp: bool, lo: u8, hi: u8) -> Vec<crate::improv::NeckDot> {
    const OPEN: [i32; 6] = [40, 45, 50, 55, 59, 64];
    let Some((root, parts)) = chord_parts(name, arp) else { return vec![] };
    let mut dots = Vec::new();
    for (s, open) in OPEN.iter().enumerate() {
        for f in lo..=hi {
            let iv = (open + f as i32 - root).rem_euclid(12);
            let on = parts.contains(&iv);
            if !on && iv != 0 {
                continue;
            }
            let (deg, col) = crate::fretboard::degree(iv, 0);
            dots.push(crate::improv::NeckDot {
                spot: (s, f),
                text: deg.to_string(),
                col,
                strength: if on { 1.0 } else { 0.35 },
                ring: false,
                root: iv == 0,
            });
        }
    }
    dots
}

/// The minutes of plan done each day, from the log (each ticked block's minutes, from that weekday's blocks).
pub(crate) fn plan_minutes(plan: &Plan, log: &BTreeMap<String, Vec<u8>>) -> BTreeMap<i64, u32> {
    let mut out = BTreeMap::new();
    for (date, done) in log {
        let Some(d) = day_number(date) else { continue };
        let blocks = day_blocks(plan, &plan.days[weekday(d)]);
        let m: u32 = done.iter().filter_map(|k| blocks.get(*k as usize)).map(|b| b.min).sum();
        if m > 0 {
            out.insert(d, m);
        }
    }
    out
}

/// Days in a row with some of the plan done, up to today (or yesterday, when today isn't started yet).
pub(crate) fn streak(mins: &BTreeMap<i64, u32>, today: i64) -> u32 {
    let mut d = if mins.contains_key(&today) { today } else { today - 1 };
    let mut n = 0;
    while mins.contains_key(&d) {
        n += 1;
        d -= 1;
    }
    n
}

impl App {
    /// A block ticked done: its minutes go into today's diary, under "My Plan: " and its name.
    fn plan_credit(&mut self, b: &Block) {
        let date = store::today_str();
        *self.store.day_mut(&date).secs.entry(format!("My Plan: {}", b.name)).or_insert(0) += b.min * 60;
        self.store_dirty = true;
    }

    /// PROGRESS: every day since you started as a square (brighter for more of the plan done; click one to open it
    /// in the diary), the days in a row, the minutes, the keepers gone solid, where you are on the ladder.
    fn plan_progress(&mut self, ui: &mut egui::Ui, plan: &Plan, week: &Week) {
        let log = self.plan.log.clone().unwrap_or_default();
        let mins = plan_minutes(plan, &log);
        let today = store::today_days();
        ui.add_space(6.0);
        section_header(ui, "PROGRESS");
        let first = mins.keys().next().copied().unwrap_or(today).min(day_number(&plan.start).unwrap_or(today)).max(today - 55);
        let total: u32 = mins.values().sum();
        let ks = self.plan.keepers().clone();
        let solid = ks.iter().filter(|k| k.solid()).count();
        let mv = self.plan.mv();
        let tiles = [
            ("IN A ROW", { let s = streak(&mins, today); format!("{} DAY{}", s, if s == 1 { "" } else { "S" }) }),
            ("DAYS PRACTISED", mins.len().to_string()),
            ("PLAN TIME", format!("{}H {}M", total / 60, total % 60)),
            ("KEEPERS SOLID", format!("{} OF {}", solid, ks.len())),
            ("RUNG", rung_now(plan, week, &mv).to_string()),
        ];
        ui.horizontal_wrapped(|ui| {
            for (name, val) in &tiles {
                let w = text_w(name, 1.0).max(text_w(val, 2.0)) + 20.0;
                let (r, _) = ui.allocate_exact_size(Vec2::new(w, 44.0), Sense::hover());
                inset(ui.painter(), r, pal().lcd);
                ptext(ui.painter(), Pos2::new(r.center().x, r.min.y + 12.0), Align::Center, name, 1.0, pal().dim);
                ptext(ui.painter(), Pos2::new(r.center().x, r.min.y + 30.0), Align::Center, val, 2.0, pal().ink);
            }
        });
        // the days as squares, a week a row (Monday first)
        let busiest = mins.values().copied().max().unwrap_or(1).max(1);
        let start_row = first - weekday(first) as i64;
        let rows = ((today - start_row) / 7 + 1) as usize;
        let cell = 26.0;
        let (gr, _) = ui.allocate_exact_size(Vec2::new(cell * 7.0 + 40.0, 16.0 + rows as f32 * cell), Sense::hover());
        for (k, n) in ["M", "T", "W", "T", "F", "S", "S"].iter().enumerate() {
            ptext(ui.painter(), Pos2::new(gr.min.x + 40.0 + k as f32 * cell + cell / 2.0, gr.min.y + 7.0), Align::Center, n, 1.0, pal().dim);
        }
        let mut open: Option<i64> = None;
        for d in start_row..=today {
            let i = (d - start_row) as usize;
            let r = Rect::from_min_size(
                Pos2::new(gr.min.x + 40.0 + (i % 7) as f32 * cell + 2.0, gr.min.y + 16.0 + (i / 7) as f32 * cell + 2.0),
                Vec2::splat(cell - 4.0),
            );
            if i % 7 == 0 {
                let (_, m, dd) = store::civil_from_days(d);
                ptext(ui.painter(), Pos2::new(gr.min.x, r.center().y), Align::Min, &format!("{}/{}", m, dd), 1.0, pal().dim);
            }
            if d < first {
                continue;
            }
            let m = mins.get(&d).copied().unwrap_or(0);
            let face = if m == 0 { pal().lcd } else { block_col("project").gamma_multiply(0.35 + 0.65 * m as f32 / busiest as f32) };
            ui.painter().rect_filled(r, 3.0, face);
            if d == today {
                ui.painter().rect_stroke(r, 3.0, Stroke::new(1.5_f32, pal().ink));
            }
            let resp = ui.interact(r, ui.id().with(("plan_day_sq", d)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            if resp.tip(format!("{}: {} min of the plan. Click to open the day in the diary (notes, times).", store::date_str(d), m)).clicked() {
                open = Some(d);
            }
        }
        if let Some(d) = open {
            self.diary_tab = 0;
            self.diary_sel = d;
            let (y, m, _) = store::civil_from_days(d);
            self.diary_ym = (y, m);
        }
        crate::views::dim_line(ui, "CLICK A DAY FOR ITS NOTES", 1.0, pal().dim);
    }

    /// A box to write a new keeper in, and ADD (Enter works too). `id`: its own per place, as two show on one page.
    fn keeper_add(&mut self, ui: &mut egui::Ui, id: u32) {
        let mut add = false;
        ui.horizontal(|ui| {
            let w = (ui.available_width() - text_w("ADD", 2.0) - 40.0).max(120.0);
            let (r, _) = ui.allocate_exact_size(Vec2::new(w, bh()), Sense::hover());
            let out = crate::views::field(ui, &mut self.ed, id, &mut self.plan.keeper_new, r, "a keeper: the lick, phrase or solo bit, and where it's from", false);
            add = out.enter;
            if retro_btn(ui, "ADD", false).tip("Keep it: Review brings it back until you can play it cold.").clicked() {
                add = true;
            }
        });
        let text = self.plan.keeper_new.trim().to_string();
        if add && !text.is_empty() {
            self.plan.keepers().push(Keeper { text, added: store::today_str(), played: vec![], seen: String::new() });
            self.plan.save_keepers();
            self.plan.keeper_new.clear();
            self.set_note("KEPT. REVIEW BRINGS IT BACK");
        }
    }

    /// Today's two keepers to play from memory: PLAYED IT COLD, or NOT YET (it comes back tomorrow).
    fn keepers_review(&mut self, ui: &mut egui::Ui) {
        let today = store::today_str();
        let ks = self.plan.keepers().clone();
        title_line(ui, "TODAY'S KEEPERS", 2.0, pal().ink);
        if ks.is_empty() {
            text_block(ui, "No keepers yet. In the fun slot, write down what's worth keeping, and it comes up here.", 2.0, pal().ink2, 6.0);
            return;
        }
        let mut hit: Option<(usize, bool)> = None;
        for i in keepers_today(&ks, &today, 2) {
            let k = &ks[i];
            let state = if k.played.contains(&today) {
                "  - PLAYED COLD TODAY"
            } else if k.seen == today {
                "  - NOT YET, BACK TOMORROW"
            } else {
                ""
            };
            text_block(ui, &format!("{}{}", k.text, state), 2.0, if k.seen == today { pal().ink2 } else { pal().ink }, 6.0);
            ui.horizontal_wrapped(|ui| {
                if retro_btn(ui, "PLAYED IT COLD", k.played.contains(&today)).tip("From memory, first go, no peeking.").clicked() {
                    hit = Some((i, true));
                }
                if retro_btn(ui, "NOT YET", false).tip("It comes back tomorrow.").clicked() {
                    hit = Some((i, false));
                }
            });
        }
        if let Some((i, cold)) = hit {
            let k = &mut self.plan.keepers()[i];
            k.seen = today.clone();
            k.played.retain(|d| *d != today);
            if cold {
                k.played.push(today.clone());
            }
            self.plan.save_keepers();
        }
    }

    /// GUIDE TONES AND ARPEGGIOS for a week's tunes: pick a tune, a chord and where on the neck; see the 3rd and
    /// 7th (or 1 3 5 7) there, the walk through the changes, and hear it.
    fn plan_guide_tones(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, wk: &Week) {
        if wk.changes.is_empty() {
            return;
        }
        ui.add_space(6.0);
        section_header(ui, "GUIDE TONES AND ARPEGGIOS");
        text_block(
            ui,
            "Guide tones are each chord's 3rd and 7th: the two notes that say which chord it is. Learn them in one area of the neck first, then the next area. Sing each one before you play it.",
            2.0,
            pal().ink2,
            6.0,
        );
        let set = self.plan.gt_set.min(wk.changes.len() - 1);
        if wk.changes.len() > 1 {
            let names: Vec<String> = wk.changes.iter().map(|c| c.name.to_uppercase()).collect();
            let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
            if let Some(i) = crate::choice_line(ui, "plan_gt_set", "TUNE", &refs, set, 320.0) {
                self.plan.gt_set = i;
                self.plan.gt_chord = 0;
            }
        }
        let ch = &wk.changes[self.plan.gt_set.min(wk.changes.len() - 1)];
        const KINDS: [&str; 2] = ["GUIDE TONES: 3RD AND 7TH", "ARPEGGIO: 1 3 5 7"];
        if let Some(i) = crate::choice_line(ui, "plan_gt_kind", "SHOW", &KINDS, (self.plan.gt_kind as usize).min(1), 320.0) {
            self.plan.gt_kind = i as u8;
        }
        let areas: Vec<&str> = AREAS.iter().map(|a| a.0).collect();
        let area = (self.plan.gt_area as usize).min(AREAS.len() - 1);
        if let Some(i) = crate::choice_line(ui, "plan_gt_area", "WHERE", &areas, area, 320.0) {
            self.plan.gt_area = i as u8;
        }
        let area = (self.plan.gt_area as usize).min(AREAS.len() - 1);
        let chords = chords_once(&ch.bars);
        if chords.is_empty() {
            return;
        }
        let pick = self.plan.gt_chord.min(chords.len() - 1);
        ui.horizontal_wrapped(|ui| {
            for (k, c) in chords.iter().enumerate() {
                if retro_btn(ui, &crate::chart::pretty_chord(c), k == pick).clicked() {
                    self.plan.gt_chord = k;
                }
            }
        });
        let pick = self.plan.gt_chord.min(chords.len() - 1);
        let arp = self.plan.gt_kind == 1;
        let (_, lo, hi) = AREAS[area];
        let dots = chord_dots(&chords[pick], arp, lo, hi);
        crate::improv::neck_view(ui, &dots, None);
        crate::views::dim_line(
            ui,
            if arp { "THE ROOT IS THE SQUARE" } else { "THE DIM SQUARES ARE THE ROOT, TO FIND THEM BY" },
            1.0,
            pal().dim,
        );
        // the changes, four bars a line
        let mut bars = String::new();
        for (k, b) in ch.bars.iter().enumerate() {
            let pretty: Vec<String> = b.split_whitespace().map(crate::chart::pretty_chord).collect();
            bars.push_str(&format!("| {} ", pretty.join(" ")));
            if k % 4 == 3 {
                bars.push_str("|\n");
            }
        }
        for l in bars.lines() {
            text_block(ui, l, 1.8, pal().ink2, 6.0);
        }
        // the walk: each chord's nearest guide tone (or arpeggio), one after another, to hear
        let flat: Vec<String> = ch.bars.iter().flat_map(|b| b.split_whitespace().map(|s| s.to_string())).collect();
        let line = crate::arpeggios::tune_line(&flat, if arp { 1 } else { 0 });
        if line.is_empty() {
            return;
        }
        let sharps = crate::theory::chord_notes(&line[0].0).map_or(false, |c| crate::lines::key_sharps(c.root));
        let gap = if arp { 60.0 / self.band_bpm() / 2.0 } else { 60.0 / self.band_bpm() * 2.0 };
        ui.horizontal_wrapped(|ui| {
            self.seq_button(
                ui,
                acts,
                if arp { "PLAY THE ARPEGGIOS" } else { "PLAY THE GUIDE TONES" },
                "Through the changes at the band's tempo, with each chord under its first note. Sing along, then play along.",
                "plangt",
                || {
                    line.iter()
                        .flat_map(|(_, root, shell, notes)| {
                            notes.iter().enumerate().map(move |(k, m)| {
                                let mut v = vec![*m];
                                if k == 0 {
                                    let bass = 40 + (root - 40).rem_euclid(12);
                                    v.push(bass);
                                    v.extend(shell.iter().map(|x| bass + x));
                                }
                                v
                            })
                        })
                        .collect()
                },
                gap,
            );
        });
        let walk: Vec<String> = line
            .iter()
            .map(|(name, root, _, notes)| {
                let degs: Vec<String> = notes
                    .iter()
                    .map(|m| format!("{} {}", crate::theory::note_name(*m, sharps), crate::fretboard::degree(m.rem_euclid(12), *root).0))
                    .collect();
                format!("{}: {}", crate::chart::pretty_chord(name), degs.join(" "))
            })
            .collect();
        text_block(ui, &walk.join("   >   "), 1.8, pal().ink, 6.0);
    }

    /// The session timer, every frame: when a block is up, a chime, it is ticked, and the next one comes (started
    /// on its own when AUTO NEXT is on).
    pub(crate) fn plan_tick(&mut self) {
        let Some(end) = self.plan.end else { return };
        let now = Instant::now();
        if now < end {
            self.ctx.request_repaint_after((end - now).min(Duration::from_millis(500)));
            return;
        }
        let Some((_, _, _, blocks)) = self.plan.today() else {
            self.plan.end = None;
            return;
        };
        let k = self.plan.block;
        if self.plan.mark(k, true) {
            self.plan_credit(&blocks[k]);
        }
        self.player.send(Cmd::Chime);
        self.ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Informational));
        if k + 1 < blocks.len() {
            let auto = self.plan.auto;
            self.plan.go(&blocks, k + 1, auto);
            self.set_note(&format!("{} DONE. NEXT: {}", blocks[k].name.to_uppercase(), blocks[k + 1].name.to_uppercase()));
        } else {
            self.plan.end = None;
            self.plan.left = 0;
            self.set_note("SESSION DONE. GOOD WORK");
        }
        self.dirty = true;
    }

    pub(crate) fn plan_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        self.plan.load();
        let plan = match &self.plan.file {
            Some(Ok(p)) => p.clone(),
            Some(Err(e)) => {
                para(ui, &format!("The plan file couldn't be read: {}", e), pal().red);
                crate::views::dim_line(ui, &file_path().display().to_string(), 1.0, pal().dim);
                return;
            }
            None => {
                para(ui, "No plan file.", pal().ink2);
                return;
            }
        };
        let Some(start) = day_number(&plan.start) else { return };
        let today = store::today_days();
        let n = plan.weeks.len() as i64;
        let now_w = week_of(start, today);
        // a new day: the session starts again from the top
        let Some((_, week_now, day, blocks)) = self.plan.today() else { return };
        if self.plan.day != today {
            self.plan.day = today;
            self.plan.end = None;
            let first = (0..blocks.len()).find(|k| !self.plan.done_today().contains(&(*k as u8))).unwrap_or(0);
            self.plan.go(&blocks, first, false);
        }

        if !plan.title.is_empty() {
            title_line(ui, &plan.title.to_uppercase(), 2.5, pal().ink);
        }
        let status = if now_w < 1 {
            format!("STARTS {}", plan.start)
        } else if now_w > n {
            format!("ALL {} WEEKS DONE", n)
        } else {
            format!("WEEK {} OF {}  -  {}", now_w, n, plan.days[weekday(today)].name.to_uppercase())
        };
        ui.horizontal_wrapped(|ui| {
            crate::views::dim_line(ui, &status, 1.0, pal().ink2);
            if retro_btn(ui, "RELOAD", false).tip("Reads the plan file again. It does this on its own when the file changes, too.").clicked() {
                self.plan.seen = None;
            }
        });

        // ---- today
        section_header(ui, &format!("TODAY  -  {}", day.name.to_uppercase()));
        if !day.note.is_empty() {
            text_block(ui, &fill(&day.note, &plan, &week_now, &day), 2.0, pal().ink, 6.0);
        }
        if blocks.is_empty() {
            text_block(ui, "Nothing on the timer today.", 2.0, pal().ink2, 6.0);
        } else {
            self.plan_session(ui, &plan, &week_now, &day, &blocks);
        }
        self.plan_progress(ui, &plan, &week_now);

        // ---- the week looked at
        let shown = (now_w.clamp(1, n) + self.plan.look).clamp(1, n);
        ui.add_space(6.0);
        section_header(ui, "THE WEEKS");
        if let Some(w) = week_strip(ui, &plan, now_w, shown) {
            self.plan.look = w - now_w.clamp(1, n);
        }
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "<", false).tip("The week before.").clicked() && shown > 1 {
                self.plan.look -= 1;
            }
            if retro_btn(ui, "THIS WEEK", self.plan.look == 0).clicked() {
                self.plan.look = 0;
            }
            if retro_btn(ui, ">", false).tip("The week after.").clicked() && shown < n {
                self.plan.look += 1;
            }
        });
        let wk = &plan.weeks[(shown - 1) as usize];
        title_line(ui, &format!("WEEK {}: {}", shown, wk.phase.to_uppercase()), 2.0, pal().ink);
        let ws = start + (shown - 1) * 7;
        let (y, m, d) = store::civil_from_days(ws);
        crate::views::dim_line(ui, &format!("STARTS {:04}-{:02}-{:02}", y, m, d), 1.0, pal().dim);
        let ink = pal().ink;
        fact(ui, "PHASE", &wk.phase, ink);
        fact(ui, "PROJECT", &wk.project, ink);
        fact(ui, "DONE WHEN", &wk.done, ink);
        fact(ui, "TUNE", &wk.tune, ink);
        fact(ui, "LINE", &wk.line, ink);
        fact(ui, "SONG", &wk.song, ink);
        if wk.rung > 0 {
            let r = plan.rungs.get(wk.rung - 1).map(|r| r.name.clone()).unwrap_or_default();
            fact(ui, "EAR LADDER", &format!("Rung {}: {}", wk.rung, r), ink);
        }
        for f in &wk.focus {
            text_block(ui, &format!("-  {}", f), 2.0, pal().ink2, 6.0);
        }
        if !wk.steps.is_empty() {
            ui.add_space(4.0);
            title_line(ui, "THE PROJECT, DAY BY DAY", 2.0, pal().ink);
            let mv = self.plan.mv();
            let on = project_at(&plan, &week_now, weekday(today), &mv);
            for (k, s) in wk.steps.iter().enumerate() {
                let name = plan.days.get(k).map(|d| short_day(&d.name)).unwrap_or_default();
                let here = on == Some((shown as usize, k));
                fact(ui, &name, &if here { format!("{}  (TODAY)", s) } else { s.clone() }, if here { pal().ink } else { pal().ink2 });
            }
        }
        let wk = wk.clone();
        self.plan_guide_tones(ui, acts, &wk);

        // ---- the keepers
        ui.add_space(6.0);
        section_header(ui, "KEEPERS");
        text_block(ui, "What's worth keeping from the fun slot. Review brings back two a day. A keeper is solid once you've played it cold on 3 days, a week apart; then it comes back every two weeks.", 2.0, pal().ink2, 6.0);
        self.keeper_add(ui, crate::tools::F_PLAN_KEEPER);
        let ks = self.plan.keepers().clone();
        let mut gone: Option<usize> = None;
        for (i, k) in ks.iter().enumerate().rev() {
            let state = if k.solid() {
                "SOLID".to_string()
            } else if k.played.is_empty() {
                "NEW".to_string()
            } else {
                format!("COLD {} DAY{}", k.played.len(), if k.played.len() == 1 { "" } else { "S" })
            };
            ui.horizontal(|ui| {
                if retro_btn(ui, "X", false).tip("Let it go.").clicked() {
                    gone = Some(i);
                }
                let (r, _) = ui.allocate_exact_size(Vec2::new(text_w("COLD 9 DAYS", 1.4) + 14.0, 22.0), Sense::hover());
                ptext(ui.painter(), Pos2::new(r.min.x + 4.0, r.center().y), Align::Min, &state, 1.4, if k.solid() { pal().ink } else { pal().dim });
                ui.vertical(|ui| text_block(ui, &k.text, 2.0, pal().ink, 0.0));
            });
        }
        if let Some(i) = gone {
            self.plan.keepers().remove(i);
            self.plan.save_keepers();
        }

        // ---- the days of a week
        ui.add_space(6.0);
        section_header(ui, "EVERY WEEK");
        let wd = weekday(today);
        let longest = plan.days.iter().map(|d| day_blocks(&plan, d).iter().map(|b| b.min).sum::<u32>()).max().unwrap_or(1);
        for (k, d) in plan.days.iter().enumerate() {
            let bl = day_blocks(&plan, d);
            let mins: u32 = bl.iter().map(|b| b.min).sum();
            let here = k == wd;
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::new(text_w("WED 999 MIN", 1.5) + 14.0, 30.0), Sense::hover());
                let c = if here { pal().ink } else { pal().ink2 };
                ptext(ui.painter(), Pos2::new(r.min.x + 4.0, r.center().y - 7.0), Align::Min, &short_day(&d.name), 1.8, c);
                ptext(ui.painter(), Pos2::new(r.min.x + 4.0, r.center().y + 8.0), Align::Min, &format!("{} MIN", mins), 1.2, pal().dim);
                if bl.is_empty() {
                    ptext(ui.painter(), Pos2::new(r.max.x + 6.0, r.center().y), Align::Min, "REST", 1.4, pal().dim);
                    ui.add_space(40.0);
                } else {
                    block_bar(ui, &format!("plan_day{}", k), &bl, longest, 30.0, None, &[]);
                }
            });
            if !d.note.is_empty() && bl.len() <= 1 {
                text_block(ui, &d.note, 1.5, pal().ink2, 6.0);
            }
        }
        // the key: every kind of block once, in its colour
        let mut kinds: Vec<(String, u32)> = Vec::new();
        for d in &plan.days {
            for b in day_blocks(&plan, d) {
                if !kinds.iter().any(|k| k.0 == b.name) {
                    kinds.push((b.name.clone(), b.min));
                }
            }
        }
        ui.horizontal_wrapped(|ui| {
            for (name, min) in &kinds {
                let txt = format!("{} {}", name, min);
                let (r, _) = ui.allocate_exact_size(Vec2::new(text_w(&txt, 1.4) + 30.0, 22.0), Sense::hover());
                let sw = Rect::from_min_size(Pos2::new(r.min.x + 2.0, r.center().y - 6.0), Vec2::splat(12.0));
                ui.painter().rect_filled(sw, 3.0, block_col(name));
                ptext(ui.painter(), Pos2::new(sw.max.x + 6.0, r.center().y), Align::Min, &txt, 1.4, pal().ink2);
            }
        });
        ui.add_space(4.0);
        title_line(ui, "THE TUNE, DAY BY DAY", 2.0, pal().ink);
        for (k, d) in plan.days.iter().enumerate() {
            if !d.tune_step.is_empty() {
                fact(ui, &short_day(&d.name), &d.tune_step, if k == wd { pal().ink } else { pal().ink2 });
            }
        }

        // ---- the ladder
        if !plan.rungs.is_empty() {
            ui.add_space(6.0);
            section_header(ui, "THE SING-THEN-PLAY LADDER");
            if !plan.ladder_about.is_empty() {
                text_block(ui, &plan.ladder_about, 2.0, pal().ink2, 6.0);
            }
            let mv = self.plan.mv();
            let cur = rung_now(&plan, &week_now, &mv);
            for (k, r) in plan.rungs.iter().enumerate() {
                let here = k + 1 == cur;
                let name = format!("RUNG {}: {}{}", k + 1, r.name.to_uppercase(), if here { "  - THIS WEEK" } else { "" });
                title_line(ui, &name, 2.0, if here { pal().ink } else { pal().ink2 });
                text_block(ui, &r.how, 2.0, if here { pal().ink } else { pal().ink2 }, 14.0);
            }
        }

        // ---- notes
        for nt in &plan.notes {
            ui.add_space(6.0);
            section_header(ui, &nt.title.to_uppercase());
            for p in &nt.points {
                text_block(ui, &format!("-  {}", p), 2.0, pal().ink, 6.0);
            }
        }
        ui.add_space(20.0);
    }

    /// Today's session: the blocks as a bar (click one to go to it), the one on now big with its time, start,
    /// pause, back, skip, a minute more, and what to do in it.
    fn plan_session(&mut self, ui: &mut egui::Ui, plan: &Plan, week: &Week, day: &Day, blocks: &[Block]) {
        let done = self.plan.done_today();
        let total: u32 = blocks.iter().map(|b| b.min).sum::<u32>().max(1);
        let cur = self.plan.block.min(blocks.len() - 1);
        let running = self.plan.end.is_some();
        // the bar: every block as wide as its minutes; click one to go to it
        let jump = block_bar(ui, "plan_seg", blocks, total, 46.0, Some(cur), &done);
        // the minutes along it
        let mins_done: u32 = blocks.iter().enumerate().filter(|(k, _)| done.contains(&(*k as u8))).map(|(_, b)| b.min).sum();
        crate::views::dim_line(ui, &format!("{} OF {} MIN DONE TODAY", mins_done, total), 1.0, pal().dim);
        if let Some(k) = jump {
            self.plan.go(blocks, k, running);
        }
        ui.add_space(6.0);
        // the block on now
        let b = &blocks[cur];
        let secs = self.plan.secs_left();
        title_line(ui, &format!("{}. {}", cur + 1, b.name.to_uppercase()), 2.5, pal().ink);
        crate::views::dim_line(
            ui,
            &format!("{} MIN  -  BLOCK {} OF {}  -  {} DONE TODAY", b.min, cur + 1, blocks.len(), done.len().min(blocks.len())),
            1.0,
            pal().ink2,
        );
        let (r, _) = ui.allocate_exact_size(Vec2::new(text_w("00:00", 6.0) + 24.0, crate::font::text_px(6.0) + 16.0), Sense::hover());
        ptext(ui.painter(), Pos2::new(r.min.x + 4.0, r.center().y), Align::Min, &mmss(secs), 6.0, if running { pal().ink } else { pal().ink2 });
        let mut act: Option<u8> = None;
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, if running { "PAUSE" } else { "START" }, running).tip("Space works too, while this tab is open.").clicked() {
                act = Some(0);
            }
            if retro_btn(ui, "BACK", false).tip("The block before.").clicked() {
                act = Some(1);
            }
            if retro_btn(ui, "SKIP", false).tip("On to the next block (this one isn't ticked).").clicked() {
                act = Some(2);
            }
            if retro_btn(ui, "DONE", done.contains(&(cur as u8))).tip("Tick this block and go on to the next.").clicked() {
                act = Some(3);
            }
            if retro_btn(ui, "+1 MIN", false).clicked() {
                act = Some(4);
            }
            if retro_btn(ui, "RESTART", false).tip("This block from the start.").clicked() {
                act = Some(5);
            }
            if retro_btn(ui, "AUTO NEXT", self.plan.auto).tip("When a block is up, start the next one on its own.").clicked() {
                act = Some(6);
            }
        });
        // space starts and pauses (when no text box has the keyboard)
        if act.is_none() && self.ed.id == 0 && ui.input(|i| i.key_pressed(egui::Key::Space)) {
            act = Some(0);
        }
        match act {
            Some(0) => {
                if running {
                    self.plan.left = self.plan.secs_left();
                    self.plan.end = None;
                } else {
                    let left = if self.plan.left == 0 { b.min as u64 * 60 } else { self.plan.left };
                    self.plan.end = Some(Instant::now() + Duration::from_secs(left));
                }
            }
            Some(1) => self.plan.go(blocks, cur.saturating_sub(1), running),
            Some(2) => {
                if cur + 1 < blocks.len() {
                    self.plan.go(blocks, cur + 1, running);
                }
            }
            Some(3) => {
                if self.plan.mark(cur, true) {
                    self.plan_credit(b);
                }
                if cur + 1 < blocks.len() {
                    self.plan.go(blocks, cur + 1, running);
                } else {
                    self.plan.end = None;
                    self.plan.left = 0;
                    self.set_note("SESSION DONE. GOOD WORK");
                }
            }
            Some(4) => match self.plan.end {
                Some(e) => self.plan.end = Some(e + Duration::from_secs(60)),
                None => self.plan.left += 60,
            },
            Some(5) => self.plan.go(blocks, cur, running),
            Some(6) => self.plan.auto = !self.plan.auto,
            _ => {}
        }
        if self.plan.end.is_some() {
            ui.ctx().request_repaint_after(Duration::from_millis(500));
        }
        ui.add_space(4.0);
        let mv = self.plan.mv();
        let week_no = plan.start_week();
        text_block(ui, &fill_moved(&b.what, plan, week, day, &mv, week_no), 2.0, pal().ink, 6.0);
        let ink2 = pal().ink2;
        fact(ui, "AHEAD?", &fill_moved(&b.ahead, plan, week, day, &mv, week_no), ink2);
        fact(ui, "BEHIND?", &fill_moved(&b.behind, plan, week, day, &mv, week_no), ink2);
        let lname = b.name.to_lowercase();
        if lname.contains("fun") {
            title_line(ui, "A KEEPER?", 2.0, pal().ink);
            self.keeper_add(ui, crate::tools::F_PLAN_KEEPER_FUN);
        } else if lname.contains("review") {
            self.keepers_review(ui);
        }
        // ahead or behind moves this block along: the rung, the project's step, the tune's step
        let Some(kind) = block_moves(b) else { return };
        let off = match kind {
            "rung" => mv.rung,
            "project" => mv.project,
            _ if mv.tune_week == week_no => mv.tune,
            _ => 0,
        };
        let (back, fwd, unit) = match kind {
            "rung" => ("DOWN A RUNG", "UP A RUNG", "RUNG"),
            _ => ("STEP BACK", "NEXT STEP", "STEP"),
        };
        let mut step: i32 = 0;
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, back, false).tip("Behind? Go back one, and the plan carries on from there.").clicked() {
                step = -1;
            }
            if retro_btn(ui, fwd, false).tip("Ahead? Move on one, and the plan carries on from there.").clicked() {
                step = 1;
            }
            if off != 0 && retro_btn(ui, "BACK ON PLAN", false).tip("Back to where the plan has you today.").clicked() {
                step = -off;
            }
            if off != 0 {
                let s = if off.abs() == 1 { "" } else { "S" };
                let txt = if off > 0 { format!("{} {}{} AHEAD OF THE PLAN", off, unit, s) } else { format!("{} {}{} BEHIND THE PLAN", -off, unit, s) };
                crate::views::dim_line(ui, &txt, 1.0, pal().ink2);
            }
        });
        if step != 0 {
            let mut m = mv.clone();
            match kind {
                "rung" => m.rung += step,
                "project" => m.project += step,
                _ => {
                    if m.tune_week != week_no {
                        m.tune = 0;
                        m.tune_week = week_no;
                    }
                    m.tune += step;
                }
            }
            self.plan.set_mv(m);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Plan {
        let day = |name: &str, core: bool| Day { name: name.into(), core, blocks: vec![], note: String::new(), tune_step: format!("{} step", name) };
        Plan {
            title: "T".into(),
            start: "2026-10-26".into(),
            blocks: vec![
                Block { name: "Warm-up".into(), min: 5, what: "Go".into(), ahead: String::new(), behind: String::new() },
                Block { name: "Tune".into(), min: 10, what: "{tune}: {tune_step}. Ladder {rung}: {rung_how}".into(), ahead: String::new(), behind: String::new() },
            ],
            days: vec![day("Mon", true), day("Tue", true), day("Wed", true), day("Thu", true), day("Fri", true), day("Sat", false), day("Sun", false)],
            weeks: vec![Week { phase: "A".into(), project: "P".into(), done: "D".into(), focus: vec![], tune: "Blue Bossa".into(), line: "L".into(), rung: 2, steps: vec![], changes: vec![], song: String::new() }],
            ladder_about: String::new(),
            rungs: vec![Rung { name: "One note".into(), how: "a".into() }, Rung { name: "Two notes".into(), how: "Sing two".into() }],
            notes: vec![],
        }
    }

    #[test]
    fn weeks_and_days_from_the_start() {
        let s = day_number("2026-10-26").unwrap();
        // 26 October 2026 is a Monday
        assert_eq!(weekday(s), 0);
        assert_eq!(weekday(s + 6), 6);
        assert_eq!(week_of(s, s - 1), 0);
        assert_eq!(week_of(s, s), 1);
        assert_eq!(week_of(s, s + 6), 1);
        assert_eq!(week_of(s, s + 7), 2);
        assert_eq!(week_of(s, s + 55), 8);
        assert!(day_number("2026-13-01").is_none() && day_number("soon").is_none());
    }

    #[test]
    fn a_block_is_filled_in_from_the_week_and_day() {
        let p = sample();
        let t = fill(&p.blocks[1].what, &p, &p.weeks[0], &p.days[1]);
        assert_eq!(t, "Blue Bossa: Tue step. Ladder rung 2, Two notes: Sing two");
        assert_eq!(day_blocks(&p, &p.days[0]).len(), 2);
        assert!(day_blocks(&p, &p.days[6]).is_empty());
    }

    #[test]
    fn going_ahead_or_behind_moves_the_block_along() {
        let mut p = sample();
        p.weeks[0].steps = vec!["a".into(), "b".into(), "c".into()];
        let mut w2 = p.weeks[0].clone();
        w2.tune = "Solar".into();
        w2.steps = vec!["d".into(), "e".into()];
        p.weeks.push(w2);
        let mut mv = Moves::default();
        let w1 = p.weeks[0].clone();
        assert_eq!(project_at(&p, &w1, 1, &mv), Some((1, 1)));
        mv.project = 2;
        assert_eq!(project_at(&p, &w1, 1, &mv), Some((2, 0)));
        assert_eq!(fill_moved("{project_step}", &p, &w1, &p.days[1], &mv, 1), "(from week 2) d");
        mv.project = 99;
        assert_eq!(project_at(&p, &w1, 1, &mv), Some((2, 1)));
        mv.project = -9;
        assert_eq!(project_at(&p, &w1, 1, &mv), Some((1, 0)));
        // the rung stays on the ladder
        mv.rung = 5;
        assert_eq!(rung_now(&p, &w1, &mv), p.rungs.len());
        mv.rung = -5;
        assert_eq!(rung_now(&p, &w1, &mv), 1);
        // the tune's step moves this week only
        mv.tune = 1;
        mv.tune_week = 1;
        assert_eq!(fill_moved("{tune_step}", &p, &w1, &p.days[0], &mv, 1), "Tue step");
        assert_eq!(fill_moved("{tune_step}", &p, &w1, &p.days[0], &mv, 2), "Mon step");
        mv.tune = 30;
        assert!(fill_moved("{tune_step}", &p, &w1, &p.days[0], &mv, 1).contains("Solar"));
        assert_eq!(block_moves(&p.blocks[1]), Some("rung"));
        assert_eq!(block_moves(&p.blocks[0]), None);
    }

    #[test]
    fn keepers_come_back_until_they_are_solid() {
        let k = |text: &str, added: &str, played: &[&str], seen: &str| Keeper {
            text: text.into(),
            added: added.into(),
            played: played.iter().map(|s| s.to_string()).collect(),
            seen: seen.into(),
        };
        let ks = vec![
            k("old seen", "2026-10-01", &["2026-10-20"], "2026-10-20"),
            k("new", "2026-10-22", &[], ""),
            k("seen yesterday", "2026-10-02", &[], "2026-10-23"),
            k("solid", "2026-09-01", &["2026-10-01", "2026-10-05", "2026-10-12"], "2026-10-20"),
        ];
        assert!(ks[3].solid() && !ks[0].solid());
        assert_eq!(keepers_today(&ks, "2026-10-24", 2), vec![1, 0]);
        // one already up today stays, and the next one fills in
        let mut ks2 = ks.clone();
        ks2[2].seen = "2026-10-24".into();
        assert_eq!(keepers_today(&ks2, "2026-10-24", 2), vec![2, 1]);
        // a solid one is back after two weeks
        assert!(keepers_today(&ks, "2026-11-10", 4).contains(&3));
        assert!(!keepers_today(&ks, "2026-10-24", 4).contains(&3));
    }

    #[test]
    fn progress_counts_the_minutes_and_the_days_in_a_row() {
        let mut p = sample();
        p.days[0].blocks = vec![Block { name: "Fun".into(), min: 40, what: String::new(), ahead: String::new(), behind: String::new() }];
        let mut log = BTreeMap::new();
        // 2026-10-26 is a Monday: warm-up 5 and fun 40 (block 2) done; the Tuesday after, the tune block (10)
        log.insert("2026-10-26".to_string(), vec![0, 2]);
        log.insert("2026-10-27".to_string(), vec![1]);
        log.insert("2026-10-29".to_string(), vec![]);
        let m = plan_minutes(&p, &log);
        let mon = day_number("2026-10-26").unwrap();
        assert_eq!(m.get(&mon), Some(&45));
        assert_eq!(m.get(&(mon + 1)), Some(&10));
        assert_eq!(m.len(), 2);
        assert_eq!(streak(&m, mon + 1), 2);
        assert_eq!(streak(&m, mon + 2), 2);
        assert_eq!(streak(&m, mon + 3), 0);
    }

    #[test]
    fn the_file_round_trips_and_is_checked() {
        let p = sample();
        let text = serde_json::to_string_pretty(&p).unwrap();
        assert_eq!(parse(&text).unwrap(), p);
        let mut short = p.clone();
        short.days.pop();
        assert!(parse(&serde_json::to_string(&short).unwrap()).is_err());
        assert_eq!(mmss(605), "10:05");
    }

    /// The private plan file, when there is one, reads (cargo test -- --ignored).
    #[test]
    #[ignore]
    fn the_private_plan_reads() {
        let t = std::fs::read_to_string(file_path()).unwrap();
        let p = parse(&t).unwrap();
        assert!(!p.weeks.is_empty());
        for w in &p.weeks {
            for c in w.changes.iter().flat_map(|c| chords_once(&c.bars)) {
                assert!(chord_parts(&c, true).is_some(), "can't read {}", c);
            }
        }
    }

    #[test]
    fn guide_tones_and_arpeggios_on_the_neck() {
        assert_eq!(chord_parts("F7", false), Some((5, vec![4, 10])));
        assert_eq!(chord_parts("Gm7", true), Some((7, vec![0, 3, 7, 10])));
        assert_eq!(chord_parts("Dm7b5", true), Some((2, vec![0, 3, 6, 10])));
        assert_eq!(chord_parts("Bdim7", false), Some((11, vec![3, 9])));
        // F7's A on the D string at the 7th fret and E flat on the G string at the 8th, lit; F (the root) dim
        let dots = chord_dots("F7", false, 5, 9);
        let at = |s: usize, f: u8| dots.iter().find(|d| d.spot == (s, f));
        assert!(at(2, 7).is_some_and(|d| d.strength == 1.0 && d.text == "3"));
        assert!(at(3, 8).is_some_and(|d| d.strength == 1.0));
        assert!(at(3, 10).is_none() && at(1, 8).is_some_and(|d| d.root && d.strength < 1.0));
        assert_eq!(chords_once(&["F7".into(), "Gm7 C7".into(), "F7 D7".into()]), vec!["F7", "Gm7", "C7", "D7"]);
    }
}
