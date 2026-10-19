//! SCALE HARMONY (under CHORDS): the chords a scale makes - major, melodic minor, harmonic minor, harmonic major -
//! one on each of its notes, as triads, 7th chords or 4ths, each with its mode and a grip. Pick one to see it on the
//! neck inside its scale, what to solo with over it (shapes from the scale, the triad pairs), and play it over the
//! chord; put two to four together as a vamp.

use super::*;
use crate::fretboard::{chord_box_labelled, degree, degree_in, grip_notes};
use crate::improv::{neck_view, NeckDot};
use crate::lines;
use crate::theory;

/// A parent scale: name, steps, the mode on each degree, the chord jazz players use it over.
pub(crate) struct Parent {
    pub name: &'static str,
    pub steps: [i32; 7],
    pub modes: [&'static str; 7],
    pub jazz: [&'static str; 7],
}

pub(crate) const PARENTS: [Parent; 4] = [
    Parent {
        name: "MAJOR",
        steps: [0, 2, 4, 5, 7, 9, 11],
        modes: ["IONIAN", "DORIAN", "PHRYGIAN", "LYDIAN", "MIXOLYDIAN", "AEOLIAN", "LOCRIAN"],
        jazz: ["\u{0394}7, 6", "-7", "-7 (\u{266D}9)", "\u{0394}7#11", "7, 7SUS", "-7\u{266D}6", "-7\u{266D}5"],
    },
    Parent {
        name: "MELODIC MINOR",
        steps: [0, 2, 3, 5, 7, 9, 11],
        modes: [
            "MELODIC MINOR",
            "DORIAN \u{266D}2",
            "LYDIAN AUGMENTED",
            "LYDIAN DOMINANT",
            "MIXOLYDIAN \u{266D}6",
            "LOCRIAN NATURAL 9",
            "ALTERED",
        ],
        jazz: ["-\u{0394}7, -6", "7SUS\u{266D}9", "\u{0394}7#5", "7#11", "7\u{266D}13", "-7\u{266D}5 (9)", "7ALT"],
    },
    Parent {
        name: "HARMONIC MINOR",
        steps: [0, 2, 3, 5, 7, 8, 11],
        modes: [
            "HARMONIC MINOR",
            "LOCRIAN NATURAL 6",
            "IONIAN #5",
            "DORIAN #4",
            "PHRYGIAN DOMINANT",
            "LYDIAN #2",
            "ALTERED \u{266D}\u{266D}7",
        ],
        jazz: [
            "-\u{0394}7",
            "-7\u{266D}5 (13)",
            "\u{0394}7#5",
            "-7#11, -6",
            "7\u{266D}9\u{266D}13",
            "\u{0394}7#11 (#9)",
            "\u{00b0}7",
        ],
    },
    Parent {
        name: "HARMONIC MAJOR",
        steps: [0, 2, 4, 5, 7, 8, 11],
        modes: [
            "HARMONIC MAJOR",
            "DORIAN \u{266D}5",
            "PHRYGIAN \u{266D}4",
            "LYDIAN \u{266D}3",
            "MIXOLYDIAN \u{266D}2",
            "LYDIAN AUGMENTED #2",
            "LOCRIAN \u{266D}\u{266D}7",
        ],
        jazz: [
            "\u{0394}7 (\u{266D}13)",
            "-7\u{266D}5 (9 13)",
            "7#9 (\u{266D}9 \u{266D}13)",
            "-\u{0394}7#11",
            "7\u{266D}9 (13)",
            "\u{0394}7#5 (#9)",
            "\u{00b0}7 (\u{266D}9)",
        ],
    },
];

const NUMERALS: [&str; 7] = ["I", "II", "III", "IV", "V", "VI", "VII"];
pub(crate) const STACKS: [&str; 3] = ["TRIADS", "7TH CHORDS", "4THS"];

/// The scale from degree `d`: its notes from that degree's root (the mode).
pub(crate) fn mode_of(steps: &[i32; 7], d: usize) -> Vec<i32> {
    (0..7).map(|k| (steps[(d + k) % 7] - steps[d]).rem_euclid(12)).collect()
}

/// The chord on degree `d`: notes from its root, ascending (a 4ths chord climbs past the octave).
pub(crate) fn stacked(steps: &[i32; 7], d: usize, stack: usize) -> Vec<i32> {
    let m = mode_of(steps, d);
    let (every, n) = if stack == 2 { (3, 3) } else { (2, if stack == 0 { 3 } else { 4 }) };
    let mut out = vec![0];
    for k in 1..n {
        let mut x = m[(k * every) % 7];
        while x <= *out.last().unwrap() {
            x += 12;
        }
        out.push(x);
    }
    out
}

/// The name Backline reads for a stacked triad or 7th chord (None for a 4ths chord).
pub(crate) fn suffix_of(chord: &[i32]) -> Option<&'static str> {
    let r: Vec<i32> = chord.iter().map(|x| x.rem_euclid(12)).collect();
    Some(match r.as_slice() {
        [0, 4, 7] => "",
        [0, 3, 7] => "m",
        [0, 3, 6] => "dim",
        [0, 4, 8] => "aug",
        [0, 4, 7, 11] => "maj7",
        [0, 3, 7, 10] => "m7",
        [0, 4, 7, 10] => "7",
        [0, 3, 6, 10] => "m7b5",
        [0, 3, 7, 11] => "m(maj7)",
        [0, 4, 8, 11] => "maj7#5",
        [0, 3, 6, 9] => "dim7",
        _ => return None,
    })
}

/// A chord name written in jazz symbols (- minor, a triangle for major 7, a circle diminished, + augmented).
pub(crate) fn jazz_name(root: &str, suffix: &str) -> String {
    let s = match suffix {
        "m" => "-",
        "dim" => "\u{00b0}",
        "aug" => "+",
        "maj7" => "\u{0394}7",
        "m7" => "-7",
        "m7b5" => "-7\u{266D}5",
        "m(maj7)" => "-\u{0394}7",
        "maj7#5" => "\u{0394}7#5",
        "dim7" => "\u{00b0}7",
        s => s,
    };
    format!("{}{}", root.replace('b', "\u{266D}"), s)
}

/// One thing to solo with over the chord: its name, a plain word on it, its notes, the line to play.
struct Solo {
    name: String,
    about: String,
    pcs: Vec<i32>,
    line: Vec<i32>,
}

/// Notes of these pitch classes climbing from `base` for two octaves, then back down.
fn up_and_down(pcs: &[i32], base: i32) -> Vec<i32> {
    let up: Vec<i32> = (base..=base + 24).filter(|m| pcs.contains(&m.rem_euclid(12))).collect();
    let mut notes = up.clone();
    notes.extend(up.iter().rev().skip(1));
    notes
}

/// The two triads one after the other, each climbing from where the last note was: up two rounds, then back.
pub(crate) fn pair_line(a: &[i32], b: &[i32], base: i32) -> Vec<i32> {
    let mut up: Vec<i32> = Vec::new();
    let mut at = base - 1;
    for _ in 0..2 {
        for t in [a, b] {
            for pc in t {
                let mut m = at + 1;
                while m.rem_euclid(12) != pc.rem_euclid(12) {
                    m += 1;
                }
                up.push(m);
                at = m;
            }
        }
    }
    let mut notes = up.clone();
    notes.extend(up.iter().rev().skip(1));
    notes
}

impl App {
    /// SCALE HARMONY: the chords of a scale, and what to play over each.
    pub(crate) fn scale_harmony(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        para(
            ui,
            "Stack a scale's notes on each of its degrees and you get its chords. Click one to see what to play over it.",
            pal().ink2,
        );
        // ---- the three choices, on one row
        let minor = |p: u8| p == 1 || p == 2;
        let sharps = lines::key_sharps(if minor(self.hm_parent) { self.hm_key + 3 } else { self.hm_key });
        ui.horizontal_wrapped(|ui| {
            let names: Vec<&str> = PARENTS.iter().map(|p| p.name).collect();
            if let Some(i) = crate::dropdown(ui, "hm_parent", "SCALE", &names, self.hm_parent as usize, 0.0) {
                self.hm_parent = i as u8;
                self.hm_vamp.clear();
            }
            let keys: Vec<&str> = (0..12).map(|pc| theory::note_name(pc, sharps)).collect();
            if let Some(i) = crate::dropdown(ui, "hm_key", "KEY", &keys, self.hm_key.rem_euclid(12) as usize, 0.0) {
                self.hm_key = i as i32;
                self.hm_vamp.clear();
            }
            if let Some(i) = crate::dropdown(ui, "hm_stack", "BUILD", &STACKS, self.hm_stack as usize, 0.0) {
                self.hm_stack = i as u8;
            }
        });
        let sharps = lines::key_sharps(if minor(self.hm_parent) { self.hm_key + 3 } else { self.hm_key });
        let parent = &PARENTS[(self.hm_parent as usize).min(PARENTS.len() - 1)];
        let stack = (self.hm_stack as usize).min(2);
        let key = self.hm_key.rem_euclid(12);
        let rs = |pc: i32| theory::note_name(pc, sharps).replace('b', "\u{266D}");

        // ---- the seven chords: (root, notes from it, name to read, name to show)
        let chords: Vec<(i32, Vec<i32>, String, String)> = (0..7)
            .map(|d| {
                let root = (key + parent.steps[d]).rem_euclid(12);
                let notes = stacked(&parent.steps, d, stack);
                let rn = theory::note_name(root, sharps);
                match suffix_of(&notes) {
                    Some(s) => (root, notes, format!("{}{}", rn, s), jazz_name(rn, s)),
                    None => {
                        let shown: Vec<String> = notes.iter().map(|x| rs(root + x)).collect();
                        (root, notes, String::new(), shown.join(" "))
                    }
                }
            })
            .collect();
        // the grips, each near the last: triads on the top three strings, 7ths as drop 2 on the top four, 4ths on A D G
        let grips: Vec<Option<[Option<u8>; 6]>> = if stack == 2 {
            chords.iter().map(|(root, n, _, _)| theory::fit_on(n, *root, &[1, 2, 3], &theory::STANDARD, 4)).collect()
        } else {
            let named: Vec<String> = chords.iter().map(|c| c.2.clone()).collect();
            let led = if stack == 0 {
                Self::led_grips(&named, 0, &[3, 4, 5], &[], 4)
            } else {
                Self::led_grips(&named, 4, &[2, 3, 4, 5], &[2], 4)
            };
            named.iter().map(|n| led.iter().find(|g| &g.0 == n).map(|g| g.1)).collect()
        };
        let pcs_of = |d: usize| -> Vec<i32> { chords[d].1.iter().map(|x| (chords[d].0 + x).rem_euclid(12)).collect() };

        // ---- the cards: click one to pick it (and hear it)
        ui.add_space(6.0);
        let sel = (self.hm_deg as usize).min(6);
        let lit_all = self.seq_now("hm_all");
        let seen = ui.available_width().min(ui.clip_rect().max.x - ui.cursor().min.x).max(140.0);
        let per = ((seen / 132.0).floor() as usize).clamp(1, 7);
        for row in (0..7).collect::<Vec<_>>().chunks(per) {
            ui.horizontal(|ui| {
                for &d in row {
                    let (r, resp) = ui.allocate_exact_size(Vec2::new(124.0, 196.0), Sense::click());
                    let p = ui.painter();
                    let on = d == sel;
                    let sounding = lit_all == Some(d);
                    if on || sounding {
                        p.rect_filled(r, 4.0, pal().bar_txt.gamma_multiply(if sounding { 0.3 } else { 0.15 }));
                    }
                    p.rect_stroke(
                        r,
                        4.0,
                        Stroke::new(if on { 2.0_f32 } else { 1.0 }, if on { pal().bar_txt } else { pal().dim }),
                    );
                    let cx = r.center().x;
                    ptext(p, Pos2::new(cx, r.min.y + 12.0), Align::Center, NUMERALS[d], 1.0, pal().dim);
                    crate::font::ptext_fit(
                        p,
                        Pos2::new(cx, r.min.y + 30.0),
                        Align::Center,
                        &chords[d].3,
                        2.0,
                        r.width() - 10.0,
                        pal().ink,
                    );
                    crate::font::ptext_fit(
                        p,
                        Pos2::new(cx, r.min.y + 50.0),
                        Align::Center,
                        parent.modes[d],
                        1.0,
                        r.width() - 8.0,
                        pal().ink2,
                    );
                    let gr = Rect::from_center_size(Pos2::new(cx, r.max.y - 66.0), Vec2::new(96.0, 116.0));
                    match grips[d] {
                        Some(g) => {
                            let (root, pcs) = (chords[d].0, pcs_of(d));
                            chord_box_labelled(p, gr, &g, &|m| degree_in(m, root, &pcs).0.to_string());
                        }
                        None => {
                            ptext(p, gr.center(), Align::Center, "NO GRIP", 1.0, pal().dim);
                        }
                    }
                    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        self.hm_deg = d as u8;
                        self.hm_solo = 0;
                        if let Some(g) = grips[d] {
                            acts.push(Action::PlayNotes(grip_notes(&g)));
                        }
                    }
                }
            });
            ui.add_space(4.0);
        }
        let beat = 60.0 / self.band_bpm();
        ui.horizontal_wrapped(|ui| {
            let all: Vec<Vec<i32>> = grips.iter().flatten().map(grip_notes).collect();
            self.seq_button(ui, acts, "HEAR ALL SEVEN", "Each chord of the scale in turn", "hm_all", || all, beat * 2.0);
        });

        // ---- the chord picked
        let (root, chord_rel, _, shown) = chords[sel].clone();
        let chord_abs = pcs_of(sel);
        let mode = mode_of(&parent.steps, sel);
        let mode_abs: Vec<i32> = mode.iter().map(|x| (root + x).rem_euclid(12)).collect();
        let chord12: Vec<i32> = chord_rel.iter().map(|x| x.rem_euclid(12)).collect();
        let deg = |pc: i32| -> String { degree_in(pc, root, &chord_abs).0.to_string() };
        section_header(ui, &format!("{}   {}   {}", NUMERALS[sel], shown, parent.modes[sel]));
        let jazz = parent.jazz[sel].split(", ").map(|s| format!("{}{}", rs(root), s)).collect::<Vec<_>>().join(" OR ");
        crate::views::dim_line(ui, &format!("PLAY THIS SOUND OVER  {}", jazz), 1.0, pal().ink);
        let named: Vec<String> = mode_abs.iter().map(|pc| format!("{} {}", rs(*pc), deg(*pc))).collect();
        crate::views::dim_line(ui, &format!("ITS SCALE:  {}", named.join("   ")), 1.0, pal().ink2);

        // ---- what to play over it: the scale, a shape from it that colours the chord, or a triad pair
        let base = |pc: i32| 52 + (pc - 4).rem_euclid(12);
        let mut solos: Vec<Solo> = vec![Solo {
            name: "THE WHOLE SCALE".to_string(),
            about: format!("All seven notes of {} {}.", rs(root), parent.modes[sel]),
            pcs: mode_abs.clone(),
            line: up_and_down(&mode_abs, base(root)),
        }];
        for (k, off) in crate::improv::si_auto(&chord12, &mode).into_iter().take(6) {
            let sr = (root + off).rem_euclid(12);
            let pcs: Vec<i32> = crate::improv::SI_SHAPES[k].1.iter().map(|x| (sr + x).rem_euclid(12)).collect();
            solos.push(Solo {
                name: format!("{} {}", rs(sr), crate::improv::SI_SHAPES[k].0),
                about: format!("A {} {} from inside the scale, played over {}.", rs(sr), crate::improv::SI_SHAPES[k].0, shown),
                line: up_and_down(&pcs, base(sr)),
                pcs,
            });
        }
        // triad pairs: the scale's triads on two neighbouring degrees (six notes, none shared), no diminished ones
        let mut pairs: Vec<(i32, usize, usize)> = Vec::new();
        for a in 0..7 {
            let b = (a + 1) % 7;
            let (ta, tb) = (stacked(&parent.steps, a, 0), stacked(&parent.steps, b, 0));
            if suffix_of(&ta) == Some("dim") || suffix_of(&tb) == Some("dim") {
                continue;
            }
            let pa: Vec<i32> = ta.iter().map(|x| (key + parent.steps[a] + x).rem_euclid(12)).collect();
            let pb: Vec<i32> = tb.iter().map(|x| (key + parent.steps[b] + x).rem_euclid(12)).collect();
            // the more colour over the chord the better; touching its 3rd or 7th keeps it anchored
            let colour = pa.iter().chain(pb.iter()).filter(|p| !chord_abs.contains(p)).count() as i32;
            let anchor = pa.iter().chain(pb.iter()).any(|p| {
                let r = (p - root).rem_euclid(12);
                chord12.contains(&r) && matches!(r, 3 | 4 | 10 | 11)
            }) as i32;
            pairs.push((colour * 2 + anchor * 3, a, b));
        }
        pairs.sort_by(|x, y| y.0.cmp(&x.0).then(x.1.cmp(&y.1)));
        for (_, a, b) in pairs.into_iter().take(3) {
            let ra = (key + parent.steps[a]).rem_euclid(12);
            let rb = (key + parent.steps[b]).rem_euclid(12);
            let (sa, sb) = (stacked(&parent.steps, a, 0), stacked(&parent.steps, b, 0));
            let ta: Vec<i32> = sa.iter().map(|x| (ra + x).rem_euclid(12)).collect();
            let tb: Vec<i32> = sb.iter().map(|x| (rb + x).rem_euclid(12)).collect();
            let na = jazz_name(theory::note_name(ra, sharps), suffix_of(&sa).unwrap_or(""));
            let nb = jazz_name(theory::note_name(rb, sharps), suffix_of(&sb).unwrap_or(""));
            let mut pcs = ta.clone();
            pcs.extend(tb.iter());
            solos.push(Solo {
                name: format!("TRIAD PAIR {} + {}", na, nb),
                about: format!("Two triads that share no notes: play {}, then {}, and keep swapping.", na, nb),
                line: pair_line(&ta, &tb, base(ra)),
                pcs,
            });
        }
        let pick = (self.hm_solo as usize).min(solos.len() - 1);
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            let names: Vec<&str> = solos.iter().map(|s| s.name.as_str()).collect();
            if let Some(i) = crate::dropdown(ui, "hm_solo", "SOLO WITH", &names, pick, 0.0) {
                self.hm_solo = i as u8;
            }
        });
        let solo = &solos[pick];
        let mut sorted: Vec<i32> = solo.pcs.clone();
        sorted.dedup();
        crate::views::dim_line(ui, &solo.about.to_uppercase(), 1.0, pal().ink2);
        let named: Vec<String> = sorted.iter().map(|pc| format!("{} {}", rs(*pc), deg(*pc))).collect();
        crate::views::dim_line(ui, &named.join("   "), 1.0, pal().ink);

        // ---- the neck: what you solo with bright (chord tones in their colours, the rest blue), the chord's
        // other notes ringed, the rest of the scale faint
        let notes = solo.line.clone();
        let tab = lines::finger_line(&notes, &theory::STANDARD);
        let lit = self.seq_now("line").and_then(|i| tab.get(i).copied().flatten());
        let blue = Color32::from_rgb(95, 168, 211);
        let mut dots: Vec<NeckDot> = Vec::new();
        for s in 0..6 {
            for f in 0..=15u8 {
                let pc = (theory::STANDARD[s] + f as i32).rem_euclid(12);
                let in_chord = chord_abs.contains(&pc);
                let dot = if solo.pcs.contains(&pc) {
                    Some((if in_chord { degree(pc, root).1 } else { blue }, 1.0, false))
                } else if in_chord {
                    Some((pal().ink2, 0.8, true))
                } else if mode_abs.contains(&pc) {
                    Some((pal().dim, 0.35, false))
                } else {
                    None
                };
                if let Some((col, strength, ring)) = dot {
                    dots.push(NeckDot { spot: (s, f), text: deg(pc), col, strength, ring, root: pc == root && strength > 0.5 });
                }
            }
        }
        neck_view(ui, &dots, lit);
        ui.horizontal_wrapped(|ui| {
            for (name, col) in [("CHORD NOTE", pal().ink), ("COLOUR NOTE", blue), ("REST OF THE SCALE", pal().dim)] {
                let (r, _) = ui.allocate_exact_size(Vec2::new(10.0, 10.0), Sense::hover());
                ui.painter().rect_filled(r, 1.0, col);
                crate::views::label(ui, name, 0.0);
                ui.add_space(10.0);
            }
        });

        // ---- play it over the chord: one row, then the music
        ui.add_space(4.0);
        let voiced: Vec<i32> = chord_rel.iter().take(4).map(|x| 48 + root + x).collect();
        self.ln_backing = Some((self.ln_over, voiced));
        let lkey = format!("hm:{}:{}:{}:{}:{}", key, self.hm_parent, stack, sel, pick);
        self.line_player(ui, acts, &notes, &lkey);
        let lit_i = self.seq_now("line");
        crate::lines_ui::line_score(ui, acts, &notes, self.ln_sub as usize, sharps, lit_i, Some(tab));

        // ---- the vamp: a few of the scale's chords, round and round with the band
        section_header(ui, "VAMP");
        let title = format!("VAMP IN {} {}", rs(key), parent.name);
        let mine = self.prac_chart.as_ref().is_some_and(|c| c.0 == title);
        ui.horizontal_wrapped(|ui| {
            let full = self.hm_vamp.len() >= 4;
            if !full && retro_btn(ui, &format!("+ ADD {}", shown), false).tip("Add the chord picked above").clicked() {
                self.hm_vamp.push(sel as u8);
            }
            let mut drop: Option<usize> = None;
            for (i, d) in self.hm_vamp.iter().enumerate() {
                let d = (*d as usize).min(6);
                if retro_btn(ui, &format!("{}  X", chords[d].3), false).tip("Take it out").clicked() {
                    drop = Some(i);
                }
            }
            if let Some(i) = drop {
                self.hm_vamp.remove(i);
            }
        });
        if self.hm_vamp.len() < 2 {
            crate::views::dim_line(
                ui,
                "PICK A CHORD ABOVE AND + ADD IT. WITH TWO TO FOUR, THE BAND LOOPS THEM, A BAR EACH.",
                1.0,
                pal().dim,
            );
        } else {
            ui.horizontal_wrapped(|ui| {
                let playing = mine && self.band_on;
                if retro_btn(ui, if playing { "STOP THE BAND" } else { "LOOP IT WITH THE BAND" }, playing)
                    .tip("The band plays the vamp round and round - the lead sheet opens with it")
                    .clicked()
                {
                    if playing {
                        acts.push(Action::Opt(crate::extras::Opt::Band));
                    } else {
                        let bars: String =
                            self.hm_vamp.iter().map(|d| format!("| {} ", chords[(*d as usize).min(6)].2)).collect();
                        self.prac_chart = Some((title.clone(), format!("T44 {}|", bars)));
                        self.chart_sel = None;
                        self.chart_tr = 0;
                        acts.push(Action::RightTab(1));
                        // a band already going starts again on the vamp
                        if self.band_on {
                            acts.push(Action::Opt(crate::extras::Opt::Band));
                        }
                        acts.push(Action::Opt(crate::extras::Opt::Band));
                    }
                }
                if retro_btn(ui, "CLEAR", false).clicked() {
                    self.hm_vamp.clear();
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_chords_of_each_scale() {
        let names = |p: usize, stack: usize| -> Vec<&str> {
            (0..7).map(|d| suffix_of(&stacked(&PARENTS[p].steps, d, stack)).unwrap_or("?")).collect()
        };
        assert_eq!(names(0, 1), ["maj7", "m7", "m7", "maj7", "7", "m7", "m7b5"]);
        assert_eq!(names(1, 1), ["m(maj7)", "m7", "maj7#5", "7", "7", "m7b5", "m7b5"]);
        assert_eq!(names(2, 1), ["m(maj7)", "m7b5", "maj7#5", "m7", "7", "maj7", "dim7"]);
        assert_eq!(names(3, 1), ["maj7", "m7b5", "m7", "m(maj7)", "7", "maj7#5", "dim7"]);
        assert_eq!(names(1, 0), ["m", "m", "aug", "", "", "dim", "dim"]);
        // 4ths: three notes a scale 4th apart (the harmonic scales have a diminished and an augmented one)
        for p in 0..4 {
            for d in 0..7 {
                let q = stacked(&PARENTS[p].steps, d, 2);
                assert!(q.windows(2).all(|w| (4..=6).contains(&(w[1] - w[0]))), "{:?}", q);
            }
        }
    }

    #[test]
    fn bb_maj7_sharp5_is_the_third_of_g_melodic_minor() {
        // G melodic minor: B\u{266D} on the III, its D triad on top; E-7\u{266D}5 on the VI
        let p = &PARENTS[1];
        let iii = stacked(&p.steps, 2, 1);
        assert_eq!(suffix_of(&iii), Some("maj7#5"));
        assert_eq!((7 + p.steps[2]) % 12, 10);
        assert_eq!(iii[1..].iter().map(|x| x - iii[1]).collect::<Vec<_>>(), [0, 4, 7]);
        assert_eq!(suffix_of(&stacked(&p.steps, 5, 1)), Some("m7b5"));
    }

    #[test]
    fn every_chord_reads_and_grips_in_every_key() {
        for p in PARENTS.iter() {
            for key in 0..12 {
                for stack in 0..2 {
                    let named: Vec<String> = (0..7)
                        .map(|d| {
                            let root = key + p.steps[d];
                            format!("{}{}", theory::note_name(root, false), suffix_of(&stacked(&p.steps, d, stack)).unwrap())
                        })
                        .collect();
                    for n in &named {
                        assert!(theory::chord_notes(n).is_some(), "{} not read", n);
                    }
                    let led = if stack == 0 {
                        App::led_grips(&named, 0, &[3, 4, 5], &[], 4)
                    } else {
                        App::led_grips(&named, 4, &[2, 3, 4, 5], &[2], 4)
                    };
                    assert_eq!(led.len(), 7, "{} in {}: {:?}", p.name, key, named);
                }
            }
        }
    }
}
