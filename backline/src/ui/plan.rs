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
pub(crate) fn fill(text: &str, plan: &Plan, week: &Week, day: &Day) -> String {
    let wd = plan.days.iter().position(|d| d.name == day.name).unwrap_or(0);
    let step = week.steps.get(wd).cloned().unwrap_or_default();
    let rung = week.rung.max(1);
    let (rname, rhow) = plan
        .rungs
        .get(rung - 1)
        .map(|r| (format!("rung {}, {}", rung, r.name), r.how.clone()))
        .unwrap_or_else(|| (format!("rung {}", rung), String::new()));
    text.replace("{tune}", &week.tune)
        .replace("{tune_step}", &day.tune_step)
        .replace("{line}", &week.line)
        .replace("{rung}", &rname)
        .replace("{rung_how}", &rhow)
        .replace("{project_step}", &step)
        .replace("{project}", &week.project)
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
}

impl Default for PlanState {
    fn default() -> PlanState {
        PlanState { file: None, seen: None, look: 0, block: 0, end: None, left: 0, auto: true, day: -1, log: None }
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

    fn done_today(&self) -> Vec<u8> {
        self.log.as_ref().and_then(|l| l.get(&store::today_str())).cloned().unwrap_or_default()
    }

    fn mark(&mut self, block: usize, on: bool) {
        let date = store::today_str();
        let log = self.log.get_or_insert_with(BTreeMap::new);
        let v = log.entry(date).or_default();
        let b = block as u8;
        if on && !v.contains(&b) {
            v.push(b);
            v.sort();
        } else if !on {
            v.retain(|x| *x != b);
        }
        if let Ok(t) = serde_json::to_string_pretty(log) {
            let _ = std::fs::write(log_path(), t);
        }
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

impl App {
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
        self.plan.mark(k, true);
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

    pub(crate) fn plan_view(&mut self, ui: &mut egui::Ui, _acts: &mut Vec<Action>) {
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
        crate::views::dim_line(ui, &status, 1.0, pal().ink2);

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
            let wd = weekday(today);
            for (k, s) in wk.steps.iter().enumerate() {
                let name = plan.days.get(k).map(|d| short_day(&d.name)).unwrap_or_default();
                let here = shown == now_w && k == wd;
                fact(ui, &name, &if here { format!("{}  (TODAY)", s) } else { s.clone() }, if here { pal().ink } else { pal().ink2 });
            }
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
            let cur = week_now.rung;
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
                self.plan.mark(cur, true);
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
        text_block(ui, &fill(&b.what, plan, week, day), 2.0, pal().ink, 6.0);
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
                Block { name: "Warm-up".into(), min: 5, what: "Go".into() },
                Block { name: "Tune".into(), min: 10, what: "{tune}: {tune_step}. Ladder {rung}: {rung_how}".into() },
            ],
            days: vec![day("Mon", true), day("Tue", true), day("Wed", true), day("Thu", true), day("Fri", true), day("Sat", false), day("Sun", false)],
            weeks: vec![Week { phase: "A".into(), project: "P".into(), done: "D".into(), focus: vec![], tune: "Blue Bossa".into(), line: "L".into(), rung: 2, steps: vec![] }],
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
    }
}
