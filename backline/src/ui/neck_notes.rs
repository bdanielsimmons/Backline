//! EAR TRAINING > NOTES ON THE NECK: naming the notes on the neck at sight. A note lights on the neck and you type
//! its name (Shift with the letter for a sharp, Ctrl for a flat, or click it), timed. It starts on the landmarks of
//! the strings picked, frets 0, 3, 5, 7, 10 and 12 (most of them known from tuning), one string at a time, each
//! landmark shown first (its name, the rule, a grip it's in) before it's asked; the next string comes in once these
//! are named right twice running. Once a string's landmarks are quick, the frets between them join on their own. Slow spots and misses come up more; the neck under the question
//! shows how quick each spot is. After each answer, the rule that gives it (fret 5 is the next string's name...).
//! Each spot's record is kept in `store.neck`; a finished session's minutes go in the practice diary.

use super::*;
use crate::store::NeckSpot;
use crate::theory;
use std::time::Instant;

/// Questions in a session.
const SESSION: u32 = 40;
/// The landmark frets.
const LANDMARKS: [u8; 6] = [0, 3, 5, 7, 10, 12];
/// A spot is quick once it's been right this many times running, in this many seconds or less (smoothed).
const QUICK_N: usize = 3;
const QUICK: f32 = 2.0;
/// The STRINGS choices: a name and the strings (from the low E, 0).
/// The strings come in in this order, one at a time (ALL SIX: the ones you know, the E strings and the A, last).
const STRING_SETS: [(&str, &[usize]); 5] = [("D G B", &[2, 3, 4]), ("D", &[2]), ("G", &[3]), ("B", &[4]), ("ALL SIX", &[2, 3, 4, 0, 1, 5])];
/// The strings you know already (the low E, the A and the high E): no showing first, and they don't wait.
const KNOWN: [usize; 3] = [0, 1, 5];
/// The order a string's landmarks are shown in: the string's own name, then the ones from tuning, then 10 and 3.
const TEACH_ORDER: [u8; 6] = [0, 12, 5, 7, 10, 3];
/// The strings by name, low E first.
const STRING_NAMES: [&str; 6] = ["low E", "A", "D", "G", "B", "high E"];
/// What the practice diary calls this time.
const DIARY_LABEL: &str = "Note names";
/// How long a wrong answer stays up before the next question (seconds); a right one.
const WRONG_WAIT: f32 = 3.0;
const RIGHT_WAIT: f32 = 0.35;

fn ok_col() -> Color32 {
    Color32::from_rgb(56, 196, 90)
}

fn bad_col() -> Color32 {
    Color32::from_rgb(235, 64, 64)
}

fn mid_col() -> Color32 {
    Color32::from_rgb(255, 170, 40)
}

/// The drill as it runs (not kept): the spot asked and when it showed, the answer given, this session.
#[derive(Default)]
pub(crate) struct NeckDrill {
    /// the spot asked: (string from the low E, fret)
    pub q: Option<(usize, u8)>,
    pub shown: Option<Instant>,
    /// the answer: its pitch class, right or not, the seconds it took; and when
    pub given: Option<(i32, bool, f32)>,
    pub given_at: Option<Instant>,
    /// this session: asked, right, seconds on the right ones, minutes spent (gaps over two minutes left out)
    pub n: u32,
    pub right: u32,
    pub quick_secs: f32,
    pub secs: f32,
    pub last: Option<Instant>,
    /// the session just finished: right, asked, average seconds, minutes
    pub done: Option<(u32, u32, f32, u32)>,
    /// strings whose frets between the landmarks have just joined (to say so)
    pub joined: Vec<usize>,
    /// LEARN THEM: the names shown on the neck, no questions
    pub learn: bool,
    /// a landmark shown before it's asked, and whether its sound has played
    pub intro: Option<(usize, u8)>,
    pub intro_sounded: bool,
    pub rng: u64,
}

/// A spot's key in the record.
fn key(s: usize, f: u8) -> String {
    format!("{s}:{f}")
}

/// The pitch class at a spot.
fn pc_at(s: usize, f: u8) -> i32 {
    (theory::STANDARD[s] + f as i32).rem_euclid(12)
}

/// A note's name the way it's usually written in jazz (Db Eb F# Ab Bb), and its other name.
fn spelled(pc: i32) -> (&'static str, Option<&'static str>) {
    match pc.rem_euclid(12) {
        0 => ("C", None),
        1 => ("D\u{266D}", Some("C#")),
        2 => ("D", None),
        3 => ("E\u{266D}", Some("D#")),
        4 => ("E", None),
        5 => ("F", None),
        6 => ("F#", Some("G\u{266D}")),
        7 => ("G", None),
        8 => ("A\u{266D}", Some("G#")),
        9 => ("A", None),
        10 => ("B\u{266D}", Some("A#")),
        _ => ("B", None),
    }
}

fn name(pc: i32) -> String {
    spelled(pc).0.to_string()
}

/// Quick at a spot: right the last few times, in about two seconds.
fn quick(sp: Option<&NeckSpot>) -> bool {
    sp.is_some_and(|sp| sp.recent.len() >= QUICK_N && sp.recent.iter().rev().take(QUICK_N).all(|r| *r) && sp.secs <= QUICK)
}

/// Learned well enough for the next string to come in: named right the last two times.
fn learned(sp: Option<&NeckSpot>) -> bool {
    sp.is_some_and(|sp| sp.recent.len() >= 2 && sp.recent.iter().rev().take(2).all(|r| *r))
}

/// A grip you already play that a landmark is in (D, G and B), to hang the name on.
pub(crate) fn hook(s: usize, f: u8) -> Option<&'static str> {
    Some(match (s, f) {
        (2, 0) => "the open D string is the root of the open D chord (x x 0 2 3 2).",
        (2, 12) => "the octave in the D barre chord at fret 10 (10 12 12 11 10 10): low E 10 is D, and this is the D above it.",
        (2, 5) => "the octave in the G barre chord at fret 3 (3 5 5 4 3 3): low E 3 is G, and this is the G above it.",
        (2, 7) => "the octave in the A barre chord at fret 5 (5 7 7 6 5 5): low E 5 is A, and this is the A above it.",
        (2, 10) => "the octave in the C barre chord at fret 8 (8 10 10 9 8 8): low E 8 is C, and this is the C above it.",
        (2, 3) => "the root of the small F chord (x x 3 2 1 1).",
        (3, 0) => "the open G string rings in the open G chord (3 2 0 0 0 3): G.",
        (3, 12) => "the octave in the G barre chord at fret 10 (x 10 12 12 12 10): A string 10 is G, and this is the G above it.",
        (3, 5) => "the octave in the C barre chord at fret 3 (x 3 5 5 5 3): A string 3 is C, and this is the C above it.",
        (3, 7) => "the octave in the D barre chord at fret 5 (x 5 7 7 7 5): A string 5 is D, and this is the D above it.",
        (3, 10) => "the octave in the F barre chord at fret 8 (x 8 10 10 10 8): A string 8 is F, and this is the F above it.",
        (3, 3) => "the octave in the B\u{266D} barre chord at fret 1 (x 1 3 3 3 1): A string 1 is B\u{266D}, and this is the B\u{266D} above it.",
        (4, 0) => "the open B string rings in open E minor (0 2 2 0 0 0): B, its 5th.",
        (4, 12) => "the top B of the D-shape B chord (x x 9 11 12 11): D string 9 is B, and this is the B above it.",
        (4, 5) => "the same E as the open high E: the fret you tune the high E to.",
        (4, 7) => "the 3rd of the D barre chord at fret 5 (x 5 7 7 7 5): F#.",
        (4, 10) => "the same A as high E fret 5: the B string is the high E five frets up.",
        (4, 3) => "the top D of the open D chord (x x 0 2 3 2).",
        _ => return None,
    })
}

/// How often a spot comes up: one not asked yet often, a slow one or one missed lately more.
fn weight(sp: Option<&NeckSpot>) -> f32 {
    match sp {
        Some(sp) if sp.asked > 0 => {
            let misses = sp.recent.iter().filter(|r| !**r).count() as f32;
            let slow = if sp.right == 0 { 4.0 } else { (sp.secs - 1.0).clamp(0.0, 5.0) };
            0.4 + slow + 2.0 * misses
        }
        _ => 3.0,
    }
}

/// The rule that gives the note at a spot: a landmark's from tuning, any other fret from the landmark nearest it.
pub(crate) fn rule(s: usize, f: u8) -> String {
    let own = name(pc_at(s, 0));
    let it = name(pc_at(s, f));
    match f {
        0 => format!("Fret 0: the string's own name, {own}."),
        12 => format!("Fret 12: the string's name again, {own}, an octave up."),
        5 => match s {
            3 => "Fret 5 on the G string is C: the B string tunes at fret 4, so 5 is one past B.".to_string(),
            5 => "Fret 5 on the high E is A, the same as on the low E.".to_string(),
            _ => format!("Fret 5 is the next string up's name, {it}: the fret you tune it at."),
        },
        7 => match s {
            4 => "Fret 7 on the B string is F#, a 5th above B (the one landmark the next string down doesn't give).".to_string(),
            0 => "Fret 7 on the low E is B, the same as on the high E: a 5th above E.".to_string(),
            _ => format!("Fret 7 is the next string down's name, {it}: the harmonic you tune with."),
        },
        10 => format!("Fret 10 is a whole step below the string's name: a whole step below {own} is {it}."),
        3 => format!("Fret 3 is a minor 3rd above the open string: {own} up a minor 3rd is {it}."),
        _ => {
            // the nearest landmark (the lower one when two are as near)
            let l = *LANDMARKS.iter().min_by_key(|&&l| ((l as i32 - f as i32).abs(), l)).unwrap_or(&0);
            let d = f as i32 - l as i32;
            let way = if d > 0 { "above" } else { "below" };
            let frets = if d.abs() == 1 { "fret" } else { "frets" };
            let mut t = format!("{} {frets} {way} the landmark at fret {l} ({}): {it}.", d.abs(), name(pc_at(s, l)));
            if s == 4 && f >= 5 {
                t += &format!(" Or: the B string is the high E five frets up, and high E fret {} is {it}.", f - 5);
            }
            t
        }
    }
}

impl App {
    /// The drill's index in EAR's menu.
    pub(crate) fn nn_drill() -> usize {
        crate::ear::DRILLS.iter().position(|d| d.key == "neck").unwrap_or(0)
    }

    /// NOTES ON THE NECK is on screen, on its page or in My Plan's block (so the letter keys are its answers).
    pub(crate) fn nn_here(&self) -> bool {
        self.mode() == 0
            && ((self.sec == Sec::Ear && self.ear.drill == Self::nn_drill()) || (self.sec == Sec::Diary && self.diary_tab == 3 && self.nn_inline))
    }

    /// A player command the drill keeps its key from: one on a letter A to G, while the drill is on screen.
    pub(crate) fn nn_owns(&self, cmd: crate::prefs::Cmd) -> bool {
        if !self.nn_here() {
            return false;
        }
        let key = crate::prefs::CMDS.iter().position(|c| c.cmd == cmd).and_then(|k| self.binds.get(k).copied().flatten());
        use egui::Key::*;
        matches!(key, Some(A | B | C | D | E | F | G))
    }

    /// The strings in play, one at a time: the next string of the ones picked comes in once every landmark of the
    /// ones before it has been named right twice running (the ones you know come in together, at the end).
    fn nn_strings_in(&self) -> Vec<usize> {
        let st = &self.store.neck;
        let mut v = Vec::new();
        for &s in STRING_SETS[st.strings.min(STRING_SETS.len() - 1)].1 {
            v.push(s);
            if !KNOWN.contains(&s) && !LANDMARKS.iter().all(|&f| learned(st.spots.get(&key(s, f)))) {
                break;
            }
        }
        v
    }

    /// The spots in play: the landmarks of the strings in play, and every fret 0 to 12 of the strings whose landmarks
    /// are quick (or of all of them with EVERY FRET).
    fn nn_pool(&self) -> Vec<(usize, u8)> {
        let st = &self.store.neck;
        let mut v = Vec::new();
        for s in self.nn_strings_in() {
            let all = st.every || st.open.contains(&s);
            for f in 0..=12u8 {
                if all || LANDMARKS.contains(&f) {
                    v.push((s, f));
                }
            }
        }
        v
    }

    /// The next question: a landmark not shown yet, shown first (its name, the rule, a grip it's in); else a spot from
    /// the pool, the slow and missed ones more often, never the same one twice running.
    fn nn_next(&mut self) {
        let new = {
            let taught = &self.store.neck.taught;
            self.nn_strings_in()
                .into_iter()
                .filter(|s| !KNOWN.contains(s))
                .flat_map(|s| TEACH_ORDER.iter().map(move |&f| (s, f)))
                .find(|&(s, f)| !taught.contains(&key(s, f)))
        };
        if let Some(spot) = new {
            self.nn.intro = Some(spot);
            self.nn.intro_sounded = false;
            self.nn.q = None;
            self.nn.given = None;
            self.nn.given_at = None;
            return;
        }
        self.nn.intro = None;
        let pool = self.nn_pool();
        if self.nn.rng == 0 {
            self.nn.rng = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(7) | 1;
        }
        let ws: Vec<f32> =
            pool.iter().map(|&(s, f)| if Some((s, f)) == self.nn.q && pool.len() > 1 { 0.0 } else { weight(self.store.neck.spots.get(&key(s, f))) }).collect();
        let total: f32 = ws.iter().sum();
        let mut x = self.nn.rng;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.nn.rng = x;
        let mut at = (x >> 40) as f32 / (1u64 << 24) as f32 * total;
        let mut pick = pool.len().saturating_sub(1);
        for (k, w) in ws.iter().enumerate() {
            if at < *w {
                pick = k;
                break;
            }
            at -= w;
        }
        self.nn.q = pool.get(pick).copied();
        self.nn.shown = Some(Instant::now());
        self.nn.given = None;
        self.nn.given_at = None;
        if self.nn.last.is_none() {
            self.nn.last = Some(Instant::now());
        }
    }

    /// An answer: right or not, timed; kept with the spot. A string whose landmarks are all quick opens its other frets.
    fn nn_answer(&mut self, pc: i32, acts: &mut Vec<Action>) {
        let Some((s, f)) = self.nn.q else { return };
        if self.nn.given.is_some() {
            return;
        }
        let secs = self.nn.shown.map_or(0.0, |t| t.elapsed().as_secs_f32());
        let ok = pc.rem_euclid(12) == pc_at(s, f);
        self.nn.given = Some((pc.rem_euclid(12), ok, secs));
        self.nn.given_at = Some(Instant::now());
        let sp = self.store.neck.spots.entry(key(s, f)).or_default();
        sp.asked += 1;
        if ok {
            sp.right += 1;
            // (smoothed: the recent answers count most; a long pause counts as ten seconds)
            sp.secs = if sp.right == 1 { secs.min(10.0) } else { sp.secs * 0.6 + secs.min(10.0) * 0.4 };
        }
        sp.recent.push(ok);
        if sp.recent.len() > 5 {
            sp.recent.remove(0);
        }
        // this session
        let now = Instant::now();
        if let Some(l) = self.nn.last {
            let d = now.duration_since(l).as_secs_f32();
            if d < 120.0 {
                self.nn.secs += d;
            }
        }
        self.nn.last = Some(now);
        self.nn.n += 1;
        if ok {
            self.nn.right += 1;
            self.nn.quick_secs += secs;
        }
        if !self.store.neck.mute {
            acts.push(Action::PlayNotes(vec![theory::STANDARD[s] + f as i32]));
        }
        // a string's other frets join once its landmarks are quick
        if !self.store.neck.every {
            for &s in STRING_SETS[self.store.neck.strings.min(STRING_SETS.len() - 1)].1 {
                if !self.store.neck.open.contains(&s) && LANDMARKS.iter().all(|&f| quick(self.store.neck.spots.get(&key(s, f)))) {
                    self.store.neck.open.push(s);
                    self.nn.joined.push(s);
                }
            }
        }
        if self.nn.n >= SESSION {
            let (right, n) = (self.nn.right, self.nn.n);
            let avg = if right > 0 { self.nn.quick_secs / right as f32 } else { 0.0 };
            let secs = self.nn.secs.round() as u32;
            let st = &mut self.store.neck;
            st.sessions.push((crate::store::today_days(), right, n, (avg * 10.0).round() / 10.0));
            if st.sessions.len() > 400 {
                st.sessions.remove(0);
            }
            if secs > 0 {
                let date = crate::store::today_str();
                *self.store.day_mut(&date).secs.entry(DIARY_LABEL.to_string()).or_insert(0) += secs;
            }
            self.nn.done = Some((right, n, avg, (secs + 30) / 60));
        }
        self.store_dirty = true;
    }

    /// A test copy on the drill (BACKLINE_NN): "learn" the names shown, "wrong" G string fret 3 answered B, "right"
    /// answered Bb, "done" a session finished.
    pub(crate) fn nn_demo(&mut self, how: &str) {
        let later = Instant::now() + std::time::Duration::from_secs(600);
        self.nn.q = Some((3, 3));
        self.nn.shown = Some(Instant::now());
        self.nn.n = 12;
        self.nn.right = 10;
        self.nn.quick_secs = 18.0;
        match how {
            "learn" => self.nn.learn = true,
            "wrong" => (self.nn.given, self.nn.given_at) = (Some((11, false, 2.4)), Some(later)),
            "right" => (self.nn.given, self.nn.given_at) = (Some((10, true, 1.6)), Some(later)),
            "done" => self.nn.done = Some((34, 40, 1.9, 4)),
            _ => {}
        }
    }

    /// A fresh session (the record stays).
    fn nn_again(&mut self) {
        let rng = self.nn.rng;
        self.nn = crate::neck_notes::NeckDrill { rng, ..Default::default() };
        self.nn_next();
    }

    /// The drill's own row of choices: which strings, landmarks or every fret, sound.
    pub(crate) fn nn_options(&mut self, ui: &mut egui::Ui) {
        let names: Vec<&str> = STRING_SETS.iter().map(|s| s.0).collect();
        let cur = self.store.neck.strings.min(STRING_SETS.len() - 1);
        if let Some(k) = crate::dropdown(ui, "nn_strings", "STRINGS", &names, cur, 0.0) {
            if k != cur {
                self.store.neck.strings = k;
                self.store_dirty = true;
                self.nn_next();
            }
        }
        if check_box(ui, "EVERY FRET", self.store.neck.every)
            .tip("All frets 0-12")
            .clicked()
        {
            self.store.neck.every = !self.store.neck.every;
            self.store_dirty = true;
            self.nn_next();
        }
        if check_box(ui, "SOUND", !self.store.neck.mute).tip("Play the note after answering").clicked() {
            self.store.neck.mute = !self.store.neck.mute;
            self.store_dirty = true;
        }
    }

    /// The drill: the neck with a note lit, your answer and the rule, the note buttons, then your neck (how quick
    /// each spot is) and the sessions. `inline`: in a block of My Plan, its choices on top.
    pub(crate) fn nn_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, inline: bool) {
        if inline {
            ui.horizontal_wrapped(|ui| {
                title_line(ui, "NOTES ON THE NECK", 2.2, pal().ink);
                ui.add_space(12.0);
                self.nn_options(ui);
            });
        }
        if self.nn.q.is_none() && self.nn.intro.is_none() {
            self.nn_next();
        }
        // (the question it's on may have left the pool, after a change of strings)
        if let Some(q) = self.nn.q {
            if !self.nn_pool().contains(&q) && self.nn.given.is_none() {
                self.nn_next();
            }
        }
        let pool = self.nn_pool();
        let strings = STRING_SETS[self.store.neck.strings.min(STRING_SETS.len() - 1)].1;
        // the keys: a letter names a note (Shift a sharp, Ctrl a flat), Enter goes on
        let keys_ok = !self.search_focus && self.ed.id == 0 && !self.panel_open() && !ui.ctx().wants_keyboard_input();
        let (typed, enter) = if keys_ok {
            ui.input(|i| {
                let mut pc = None;
                for (k, p) in [(egui::Key::C, 0), (egui::Key::D, 2), (egui::Key::E, 4), (egui::Key::F, 5), (egui::Key::G, 7), (egui::Key::A, 9), (egui::Key::B, 11)] {
                    if i.key_pressed(k) {
                        let m = i.modifiers;
                        pc = Some(p + if m.shift { 1 } else if m.ctrl || m.command { -1 } else { 0 });
                    }
                }
                // (Ctrl+C comes as a copy, not a key: C flat)
                if pc.is_none() && i.events.iter().any(|e| matches!(e, egui::Event::Copy)) {
                    pc = Some(-1);
                }
                (pc, i.key_pressed(egui::Key::Enter))
            })
        } else {
            (None, false)
        };
        ui.add_space(6.0);
        if self.nn.learn {
            self.nn_learn(ui, strings);
            return;
        }
        if let Some((right, n, avg, mins)) = self.nn.done {
            // the session's end: how it went, the slowest spots, again
            title_line(ui, &format!("{right} OF {n} RIGHT"), 2.4, pal().ink);
            let mut said = format!("{avg:.1} seconds a note on average, {mins} min. ");
            let mut slow: Vec<(usize, u8, f32)> =
                pool.iter().filter_map(|&(s, f)| self.store.neck.spots.get(&key(s, f)).map(|sp| (s, f, if sp.right == 0 { 99.0 } else { sp.secs }))).collect();
            slow.sort_by(|a, b| b.2.total_cmp(&a.2));
            let slow: Vec<String> = slow.iter().take(3).map(|(s, f, _)| format!("{} string fret {f}", STRING_NAMES[*s])).collect();
            if !slow.is_empty() {
                said += &format!("Slowest now: {}.", slow.join(", "));
            }
            para(ui, &said, pal().ink2);
            ui.add_space(8.0);
            if retro_btn(ui, "AGAIN", false).tip("Another 40  (Enter)").clicked() || enter {
                self.nn_again();
            }
            ui.add_space(14.0);
            self.nn_map(ui, &pool);
            return;
        }
        // a landmark shown before it's asked: its name, the rule, a grip it's in; Enter, GOT IT or its name goes on
        if let Some((s, f)) = self.nn.intro {
            if !self.nn.intro_sounded {
                self.nn.intro_sounded = true;
                if !self.store.neck.mute {
                    acts.push(Action::PlayNotes(vec![theory::STANDARD[s] + f as i32]));
                }
            }
            let k = TEACH_ORDER.iter().position(|&x| x == f).map_or(1, |k| k + 1);
            crate::views::dim_line(ui, &format!("NEW: THE {} STRING  ·  LANDMARK {k} OF 6", STRING_NAMES[s].to_uppercase()), 1.2, pal().ink2);
            ui.add_space(4.0);
            let w = ui.available_width().min(860.0);
            let (r, _) = ui.allocate_exact_size(Vec2::new(w, 6.0 * 26.0 + 40.0), Sense::hover());
            neck_drawing(ui, r, strings, &[(s, f, None)], true, None);
            let (ar, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 40.0), Sense::hover());
            let (it, other) = spelled(pc_at(s, f));
            let it = match other {
                Some(o) => format!("{it} ({o})"),
                None => it.to_string(),
            };
            let head = format!("{} STRING, FRET {f}:  {it}", STRING_NAMES[s].to_uppercase());
            crate::font::keep_caps(|| ptext(ui.painter(), Pos2::new(ar.min.x + 6.0, ar.center().y), Align::Min, &head, 2.6, pal().ink));
            para(ui, &rule(s, f), pal().ink2);
            if let Some(h) = hook(s, f) {
                para(ui, &format!("In a grip: {h}"), pal().ink2);
            }
            ui.add_space(6.0);
            let mut got = enter || typed.is_some_and(|pc: i32| pc.rem_euclid(12) == pc_at(s, f));
            ui.horizontal(|ui| {
                if retro_btn(ui, "GOT IT", false).tip("Next note  (Enter)").clicked() {
                    got = true;
                }
                ui.add_space(8.0);
                let (tr, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), bh()), Sense::hover());
                ptext(ui.painter(), Pos2::new(tr.min.x, tr.center().y), Align::Min, "Say it, then Enter (or type its name)", 1.5, pal().ink2);
            });
            if got {
                self.store.neck.taught.push(key(s, f));
                self.store_dirty = true;
                self.nn_next();
                ui.ctx().request_repaint();
            }
            ui.add_space(14.0);
            self.nn_map(ui, &pool);
            return;
        }
        // on to the next question once the answer has been seen
        if let (Some((_, ok, _)), Some(t)) = (self.nn.given, self.nn.given_at) {
            let wait = if ok { RIGHT_WAIT } else { WRONG_WAIT };
            if t.elapsed().as_secs_f32() >= wait || (enter && !ok) {
                self.nn_next();
            } else {
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(60));
            }
        }
        if let Some(pc) = typed {
            if self.nn.given.is_none() {
                self.nn_answer(pc, acts);
                ui.ctx().request_repaint();
            }
        }
        let Some((s, f)) = self.nn.q else { return };
        // where we are in the session
        let avg = if self.nn.right > 0 { format!("  ·  {:.1} S A NOTE", self.nn.quick_secs / self.nn.right as f32) } else { String::new() };
        crate::views::dim_line(ui, &format!("{} OF {SESSION}  ·  {} RIGHT{avg}", self.nn.n, self.nn.right), 1.2, pal().ink2);
        ui.add_space(4.0);
        // the neck, the note lit
        let given = self.nn.given;
        let w = ui.available_width().min(860.0);
        let (r, _) = ui.allocate_exact_size(Vec2::new(w, 6.0 * 26.0 + 40.0), Sense::hover());
        neck_drawing(ui, r, strings, &[(s, f, given.map(|g| g.1))], given.is_some(), None);
        // the answer, and the rule that gives it
        let (ar, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 40.0), Sense::hover());
        let p = ui.painter();
        let x = ar.min.x + 6.0;
        match given {
            None => {
                ptext(p, Pos2::new(x, ar.center().y), Align::Min, &format!("{} STRING, FRET {f}:  WHAT NOTE?", STRING_NAMES[s].to_uppercase()), 2.4, pal().ink);
            }
            Some((g, ok, secs)) => {
                let (it, other) = spelled(pc_at(s, f));
                let it = match other {
                    Some(o) => format!("{it} ({o})"),
                    None => it.to_string(),
                };
                let text = if ok { format!("{it}   {secs:.1} S") } else { format!("{it}  -  NOT {}", name(g)) };
                crate::font::keep_caps(|| ptext(p, Pos2::new(x, ar.center().y), Align::Min, &text, 3.0, if ok { ok_col() } else { bad_col() }));
            }
        }
        let hint = if given.is_some() { rule(s, f) } else { "Type the letter (Shift for sharp, Ctrl for flat) or click below.".to_string() };
        para(ui, &hint, pal().ink2);
        // the notes to click
        ui.add_space(4.0);
        let mut clicked = None;
        ui.horizontal_wrapped(|ui| {
            for pc in 0..12 {
                let (a, b) = spelled(pc);
                let label = match b {
                    Some(b) => format!("{a} {b}"),
                    None => a.to_string(),
                };
                let lit = given.is_some_and(|g| g.0 == pc);
                if crate::font::keep_caps(|| retro_btn_w(ui, &label, if b.is_some() { 70.0 } else { 44.0 }, lit)).clicked() {
                    clicked = Some(pc);
                }
            }
        });
        if let Some(pc) = clicked {
            if self.nn.given.is_none() {
                self.nn_answer(pc, acts);
                ui.ctx().request_repaint();
            }
        }
        // a string's other frets just joined
        if !self.nn.joined.is_empty() {
            let names: Vec<&str> = self.nn.joined.iter().map(|&s| STRING_NAMES[s]).collect();
            ui.add_space(6.0);
            para(ui, &format!("Your landmarks on the {} string are quick: its other frets are in now.", names.join(" and ")), ok_col());
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if retro_btn(ui, "LEARN THEM", false).tip("Show note names and rules").clicked() {
                self.nn.learn = true;
            }
        });
        ui.add_space(14.0);
        self.nn_map(ui, &pool);
    }

    /// LEARN THEM: the notes in play named on the neck, and the rules for the landmarks.
    fn nn_learn(&mut self, ui: &mut egui::Ui, strings: &[usize]) {
        let pool = self.nn_pool();
        let w = ui.available_width().min(860.0);
        let (r, _) = ui.allocate_exact_size(Vec2::new(w, 6.0 * 26.0 + 40.0), Sense::hover());
        let spots: Vec<(usize, u8, Option<bool>)> = pool.iter().map(|&(s, f)| (s, f, None)).collect();
        neck_drawing(ui, r, strings, &spots, true, None);
        ui.add_space(6.0);
        for t in [
            "Frets 0 and 12: the string's own name.",
            "Fret 5: the next string up's name, the fret you tune it at. On the G string it's C, because the B string tunes at fret 4.",
            "Fret 7: the next string down's name, the harmonic you tune with. On the B string it's F#.",
            "Fret 10: a whole step below the string's name.",
            "Fret 3: a minor 3rd above the open string.",
            "Every other fret is a fret or two from one of these. The B string is the high E five frets up.",
        ] {
            para(ui, t, pal().ink2);
        }
        ui.add_space(8.0);
        if retro_btn(ui, "BACK TO THE QUESTIONS", false).clicked() {
            self.nn.learn = false;
            self.nn_next();
        }
    }

    /// YOUR NECK: each spot in play, coloured by how quick you name it (green quick, orange getting there, red slow
    /// or missed lately, an empty ring not asked yet), and the last sessions.
    fn nn_map(&mut self, ui: &mut egui::Ui, pool: &[(usize, u8)]) {
        section_header(ui, "YOUR NECK");
        let strings = STRING_SETS[self.store.neck.strings.min(STRING_SETS.len() - 1)].1;
        let w = ui.available_width().min(860.0);
        let (r, _) = ui.allocate_exact_size(Vec2::new(w, 6.0 * 18.0 + 34.0), Sense::hover());
        let heat: Vec<(usize, u8, Color32)> = pool
            .iter()
            .map(|&(s, f)| {
                let sp = self.store.neck.spots.get(&key(s, f));
                let col = match sp {
                    None => Color32::TRANSPARENT,
                    Some(sp) if sp.asked == 0 => Color32::TRANSPARENT,
                    sp if quick(sp) => ok_col(),
                    Some(sp) if sp.right > 0 && sp.secs <= 3.5 && sp.recent.iter().filter(|r| !**r).count() <= 1 => mid_col(),
                    _ => bad_col(),
                };
                (s, f, col)
            })
            .collect();
        neck_drawing(ui, r, strings, &[], false, Some(&heat));
        crate::views::dim_line(ui, "GREEN QUICK  ·  ORANGE GETTING THERE  ·  RED SLOW OR MISSED LATELY  ·  RING NOT ASKED YET", 1.0, pal().ink2);
        let sessions = &self.store.neck.sessions;
        if !sessions.is_empty() {
            let last: Vec<String> = sessions.iter().rev().take(6).map(|(_, r, n, a)| format!("{r}/{n} at {a:.1} s")).collect();
            crate::views::dim_line(ui, &format!("LAST SESSIONS, NEWEST FIRST:  {}", last.join("   ")), 1.0, pal().ink2);
        }
    }
}

/// The neck for the drill, high E on top, frets 0 to 12: the strings in play bold, the others thin. `spots` are notes
/// lit (named when `names`; right or wrong once answered); `heat` is small coloured dots instead (no names).
fn neck_drawing(ui: &egui::Ui, r: Rect, strings: &[usize], spots: &[(usize, u8, Option<bool>)], names: bool, heat: Option<&[(usize, u8, Color32)]>) {
    let p = ui.painter_at(r);
    let small = heat.is_some();
    let gap = if small { 18.0 } else { 26.0 };
    let label_w = 54.0;
    let nut = r.min.x + label_w + 42.0;
    let fw = (r.max.x - nut - 8.0) / 12.0;
    // (room above the top string for its dots, so they aren't cut off)
    let top = r.min.y + 16.0;
    let y = |s: usize| top + (5 - s) as f32 * gap;
    let x = |f: u8| if f == 0 { nut - 18.0 } else { nut + (f as f32 - 0.5) * fw };
    p.rect_filled(Rect::from_min_max(Pos2::new(nut, top - 8.0), Pos2::new(nut + 12.0 * fw, y(0) + 8.0)), 2.0, pal().display);
    for k in 0..=12 {
        let fx = nut + k as f32 * fw;
        let wire = pal().ink2.gamma_multiply(if k == 0 { 0.9 } else { 0.4 });
        p.line_segment([Pos2::new(fx, top - 6.0), Pos2::new(fx, y(0) + 6.0)], Stroke::new(if k == 0 { 4.0_f32 } else { 1.2 }, wire));
    }
    for k in [3u8, 5, 7, 9, 12] {
        let cx = x(k);
        let col = pal().ink2.gamma_multiply(0.35);
        if k == 12 {
            p.circle_filled(Pos2::new(cx, y(4) - gap * 0.5), 3.5, col);
            p.circle_filled(Pos2::new(cx, y(1) + gap * 0.5), 3.5, col);
        } else {
            p.circle_filled(Pos2::new(cx, (y(3) + y(2)) / 2.0), 3.5, col);
        }
        ptext(&p, Pos2::new(cx, y(0) + if small { 17.0 } else { 22.0 }), Align::Center, &k.to_string(), if small { 1.2 } else { 1.5 }, pal().ink2);
    }
    for s in 0..6 {
        let on = strings.contains(&s);
        p.line_segment([Pos2::new(nut, y(s)), Pos2::new(nut + 12.0 * fw, y(s))], Stroke::new(if on { 1.6_f32 } else { 1.0 }, pal().ink2));
        let open = theory::note_name(theory::STANDARD[s], true);
        ptext(&p, Pos2::new(r.min.x + label_w, y(s)), Align::Max, &open, if small { 1.2 } else { 1.6 }, if on { pal().ink } else { pal().ink2 });
    }
    if let Some(heat) = heat {
        for &(s, f, col) in heat {
            let c = Pos2::new(x(f), y(s));
            if col == Color32::TRANSPARENT {
                p.circle_stroke(c, 5.5, Stroke::new(1.0_f32, pal().ink2.gamma_multiply(0.7)));
            } else {
                p.circle_filled(c, 6.0, col);
            }
        }
        return;
    }
    for &(s, f, ok) in spots {
        let c = Pos2::new(x(f), y(s));
        let col = match ok {
            Some(true) => ok_col(),
            Some(false) => bad_col(),
            None if names => Color32::from_gray(235),
            // (its own blue: on some looks the accent is grey)
            None => Color32::from_rgb(80, 165, 255),
        };
        p.circle_filled(c, 12.0, col);
        if names {
            let n = name(pc_at(s, f));
            crate::font::keep_caps(|| ptext(&p, c, Align::Center, &n, if n.chars().count() > 1 { 1.3 } else { 1.6 }, Color32::from_gray(15)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_landmarks_are_the_notes_the_rules_say() {
        // the table: D, G and B at frets 0 3 5 7 10 12
        let want = [(2, ["D", "F", "G", "A", "C", "D"]), (3, ["G", "B\u{266D}", "C", "D", "F", "G"]), (4, ["B", "D", "E", "F#", "A", "B"])];
        for (s, names) in want {
            for (k, f) in LANDMARKS.iter().enumerate() {
                assert_eq!(name(pc_at(s, *f)), names[k], "string {s} fret {f}");
            }
        }
        // fret 5 is the next string up's name (not on G), fret 7 the next string down's (not on B or the low E)
        for s in [0, 1, 2, 4] {
            assert_eq!(pc_at(s, 5), pc_at(s + 1, 0));
        }
        for s in [1, 2, 3, 5] {
            assert_eq!(pc_at(s, 7), pc_at(s - 1, 0));
        }
        // every rule names the note it's about
        for s in 0..6 {
            for f in 0..=12u8 {
                let r = rule(s, f);
                assert!(r.contains(&name(pc_at(s, f))), "{s} {f}: {r}");
            }
        }
    }

    #[test]
    fn every_landmark_on_d_g_and_b_has_a_grip_naming_it() {
        for s in [2, 3, 4] {
            for f in LANDMARKS {
                let h = hook(s, f).unwrap_or_else(|| panic!("no grip for {s} {f}"));
                assert!(h.contains(&name(pc_at(s, f))), "{s} {f}: {h}");
            }
        }
    }

    #[test]
    fn slow_and_missed_spots_come_up_more() {
        let quick_one = NeckSpot { asked: 5, right: 5, secs: 1.0, recent: vec![true; 5] };
        let slow_one = NeckSpot { asked: 5, right: 5, secs: 4.0, recent: vec![true; 5] };
        let missed = NeckSpot { asked: 5, right: 3, secs: 1.5, recent: vec![true, false, true, false, true] };
        assert!(weight(Some(&slow_one)) > weight(Some(&quick_one)));
        assert!(weight(Some(&missed)) > weight(Some(&quick_one)));
        assert!(weight(None) > weight(Some(&quick_one)));
        assert!(quick(Some(&quick_one)) && !quick(Some(&slow_one)) && !quick(Some(&missed)));
    }
}
