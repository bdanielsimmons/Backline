//! CHORD MELODIES (under PRACTICE): SONGS - your transcriptions bar by bar (chord names, grips, the notes on a staff,
//! tab, note names), played at any tempo with the beat sounding lit, any bars looped, any beat opened in the chord
//! analyzer - and MOVES: the moves Backline finds in them (ii-V-Is, diminished passing chords, tritone subs,
//! chromatic approach chords, inner lines under a held top note, walking bass) and the ones you keep, each played,
//! transposed, starred and named.

use super::*;
use crate::transcription::{self, Beat, Move, Tune};

/// What CHORD MELODIES shows: a tune's beats laid out, its suggested moves.
type Spot = (usize, usize); // (bar, beat)

/// The colours of a beat's chord tones by degree (the analyzer's).
fn chord_root(name: &str) -> Option<i32> {
    let (r, _) = crate::chart::split_root(name.trim());
    crate::chart::note_index(r)
}

/// A move's kind, as offered in the list.
/// Short names and colours for the strips marking the moves over the bars.
const SHORT: [&str; 10] = [
    "II-V-I",
    "DIM PASSING",
    "TRITONE SUB",
    "APPROACH",
    "INNER LINE",
    "WALKING BASS",
    "GRIP MOVE",
    "SMOOTH",
    "MINE",
    "FROM THE GRIP",
];
const MARK_COLS: [(u8, u8, u8); 10] = [
    (176, 64, 52),
    (120, 72, 150),
    (40, 110, 160),
    (196, 120, 30),
    (60, 130, 80),
    (110, 90, 60),
    (170, 60, 120),
    (40, 140, 140),
    (90, 90, 90),
    (130, 150, 40),
];

/// What each kind of move is, in a line.
const EXPLAIN: [&str; 10] = [
    "A II-V-I: minor 7th, dominant, home - down a 5th each time.",
    "A diminished chord a half step between two chords, leading up into the next.",
    "A tritone sub: the dominant swapped for the one a tritone away (same 3rd and 7th), sliding down a half step home.",
    "A chord a half step above or below the target, sliding into it.",
    "The top note held while the voices under it move.",
    "The bass walking under the chords - stepping between the roots.",
    "A grip held while notes inside it move - the fingers that stay are the anchor (lit: the moving notes).",
    "Every voice of one grip moves a step or less into the next chord - the shortest way between them.",
    "One you kept yourself.",
    "After the chord stab, the melody stays where the hand is - notes the grip puts in reach (lit: the melody).",
];

/// The kind a move you pick yourself gets (kept at 8: the kinds after it came later).
const MY_OWN: usize = 8;

pub(crate) const KINDS: [&str; 10] = [
    "II-V-I",
    "DIMINISHED PASSING CHORD",
    "TRITONE SUB",
    "CHROMATIC APPROACH CHORD",
    "INNER LINE UNDER A HELD TOP NOTE",
    "WALKING BASS UNDER CHORDS",
    "GRIP MOVE (INNER NOTES MOVING IN A GRIP)",
    "SMOOTH CONNECTION INTO THE NEXT CHORD",
    "MY OWN",
    "MELODY FROM THE GRIP (NOTES NEAR A CHORD STAB)",
];

/// The moves a tune holds, found from its chord names and notes: (kind, first beat, last beat).
pub(crate) fn suggest(t: &Tune) -> Vec<(usize, Spot, Spot)> {
    // every beat in order, with its chord (the last one named, carried on)
    let mut flat: Vec<(Spot, Option<String>, &Beat)> = Vec::new();
    for (bi, b) in t.bars.iter().enumerate() {
        for (ki, k) in b.beats.iter().enumerate() {
            flat.push(((bi, ki), k.chord.clone(), k));
        }
    }
    // the chord changes: (where, name, root, quality text)
    let changes: Vec<(Spot, String, i32, String)> = flat
        .iter()
        .filter_map(|(s, c, _)| {
            let c = c.as_ref()?;
            let r = chord_root(c)?;
            let q = crate::chart::split_root(c.trim()).1.to_lowercase();
            Some((*s, c.clone(), r, q))
        })
        .collect();
    let minor = |q: &str| q.starts_with('m') && !q.starts_with("maj") || q.starts_with('-');
    let dom = |q: &str| q.starts_with('7') || q.starts_with('9') || q.starts_with("13") || q.contains("alt");
    let dim = |q: &str| q.contains("dim") || q.contains('o') || q.contains('°');
    let mut out: Vec<(usize, Spot, Spot)> = Vec::new();
    for w in changes.windows(3) {
        let (a, b, c) = (&w[0], &w[1], &w[2]);
        // ii-V-I: down a 5th twice, minor then dominant
        if minor(&a.3) && !dim(&a.3) && dom(&b.3) && (a.2 - b.2).rem_euclid(12) == 7 && (b.2 - c.2).rem_euclid(12) == 7 {
            out.push((0, a.0, c.0));
        }
        // a diminished chord a half step between two chords a whole step apart
        if dim(&b.3) && (b.2 - a.2).rem_euclid(12) == 1 && (c.2 - b.2).rem_euclid(12) == 1 {
            out.push((1, a.0, c.0));
        }
    }
    for w in changes.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        // a dominant a half step above where it goes: the tritone sub of its V
        if dom(&a.3) && (a.2 - b.2).rem_euclid(12) == 1 {
            out.push((2, a.0, b.0));
        } else if !dim(&a.3) && ((a.2 - b.2).rem_euclid(12) == 1 || (b.2 - a.2).rem_euclid(12) == 1) && a.3 == b.3 {
            // the same chord a half step away, sliding into the next
            out.push((3, a.0, b.0));
        }
    }
    // an inner line under a held top note: 3+ beats in a row, the same top note, the notes under it moving
    let mut run: Vec<usize> = Vec::new();
    let top = |b: &Beat| b.notes.iter().map(|n| n[0]).max();
    for i in 0..flat.len() {
        let same = run
            .last()
            .is_some_and(|j| top(flat[*j].2).is_some() && top(flat[*j].2) == top(flat[i].2) && flat[i].2.notes.len() >= 2);
        if same {
            run.push(i);
        } else {
            if run.len() >= 3 {
                let moved = run.windows(2).any(|w| flat[w[0]].2.notes != flat[w[1]].2.notes);
                if moved {
                    out.push((4, flat[run[0]].0, flat[*run.last().unwrap()].0));
                }
            }
            run = if flat[i].2.notes.len() >= 2 { vec![i] } else { Vec::new() };
        }
    }
    // walking bass: 4+ beats in a row whose lowest notes move by step (1 or 2 semitones), under chords
    let low = |b: &Beat| b.notes.iter().map(|n| n[0]).min();
    let mut walk: Vec<usize> = Vec::new();
    for i in 0..flat.len() {
        let ok = flat[i].2.notes.len() >= 2
            && walk.last().is_some_and(|j| match (low(flat[*j].2), low(flat[i].2)) {
                (Some(a), Some(b)) => (1..=2).contains(&(a - b).abs()),
                _ => false,
            });
        if ok {
            walk.push(i);
        } else {
            if walk.len() >= 4 {
                out.push((5, flat[walk[0]].0, flat[*walk.last().unwrap()].0));
            }
            walk = if flat[i].2.notes.len() >= 2 { vec![i] } else { Vec::new() };
        }
    }
    // a grip held while notes move inside it: what rings at each beat (struck now, or still held from before - the
    // latest on each string), two or more fretted notes staying put while others come and go within the hand's
    // reach, the moving notes changing at least twice
    let mut times: Vec<(f32, f32)> = Vec::with_capacity(flat.len());
    let mut q = 0.0f32;
    for b in &t.bars {
        for k in &b.beats {
            times.push((q + k.at, q + k.at + k.len));
        }
        q += b.time.0 as f32 * 4.0 / b.time.1.max(1) as f32;
    }
    let ring: Vec<Vec<(i32, i32)>> = (0..flat.len())
        .map(|i| {
            let s = times[i].0;
            let mut v: Vec<(i32, i32)> = Vec::new();
            for j in (i.saturating_sub(16)..=i).rev() {
                if j != i && times[j].1 <= s + 0.01 {
                    continue;
                }
                for n in &flat[j].2.notes {
                    if !v.iter().any(|p| p.0 == n[1]) {
                        v.push((n[1], n[2]));
                    }
                }
            }
            v
        })
        .collect();
    let mut i = 0;
    while i < flat.len() {
        let mut held: Vec<(i32, i32)> = ring[i].iter().filter(|p| p.1 > 0).copied().collect();
        let mut j = i + 1;
        while j < flat.len() && j - i < 24 {
            let next: Vec<(i32, i32)> = held.iter().filter(|p| ring[j].contains(p)).copied().collect();
            if next.len() < 2 {
                break;
            }
            held = next;
            j += 1;
        }
        let mut found = false;
        if held.len() >= 2 && j - i >= 3 {
            let lo = held.iter().map(|p| p.1).min().unwrap_or(0);
            let hi = held.iter().map(|p| p.1).max().unwrap_or(0);
            let moving: Vec<Vec<(i32, i32)>> = (i..j)
                .map(|k| {
                    let mut m: Vec<(i32, i32)> = ring[k].iter().filter(|p| !held.contains(p)).copied().collect();
                    m.sort();
                    m
                })
                .collect();
            let in_reach = moving.iter().flatten().all(|p| p.1 == 0 || (p.1 >= lo - 1 && p.1 <= hi + 2));
            let changes = moving.windows(2).filter(|w| w[0] != w[1]).count();
            if in_reach && changes >= 2 {
                out.push((6, flat[i].0, flat[j - 1].0));
                found = true;
            }
        }
        i = if found { j } else { i + 1 };
    }
    // a melody from the grip: after a chord stab, three or more melody notes where the hand already is (inside the
    // grip's frets, give or take one), at least one of them not in the grip - the notes it puts within reach
    let mut i = 0;
    while i < flat.len() {
        let struck: Vec<&[i32; 4]> = flat[i].2.notes.iter().filter(|n| n[3] == 0).collect();
        let fretted: Vec<i32> = struck.iter().map(|n| n[2]).filter(|f| *f > 0).collect();
        if struck.len() < 3 || fretted.len() < 2 {
            i += 1;
            continue;
        }
        let (lo, hi) = (fretted.iter().min().copied().unwrap_or(0), fretted.iter().max().copied().unwrap_or(0));
        let grip: Vec<(i32, i32)> = struck.iter().map(|n| (n[1], n[2])).collect();
        let (mut j, mut notes, mut outside) = (i + 1, 0, false);
        while j < flat.len() && j - i < 16 {
            let s: Vec<&[i32; 4]> = flat[j].2.notes.iter().filter(|n| n[3] == 0).collect();
            if s.is_empty() {
                j += 1;
                continue;
            }
            if s.len() >= 3 || !s.iter().all(|n| n[2] == 0 || (n[2] >= lo - 1 && n[2] <= hi + 1)) {
                break;
            }
            notes += s.len();
            outside |= s.iter().any(|n| !grip.contains(&(n[1], n[2])));
            j += 1;
        }
        if notes >= 3 && outside {
            out.push((9, flat[i].0, flat[j - 1].0));
            i = j;
        } else {
            i += 1;
        }
    }
    // one chord's grip into the next with every voice moving a whole step or less
    let named: Vec<usize> = (0..flat.len()).filter(|i| flat[*i].1.is_some() && flat[*i].2.notes.len() >= 3).collect();
    for w in named.windows(2) {
        let (a, b) = (flat[w[0]].2, flat[w[1]].2);
        if a.notes.len() == b.notes.len() && flat[w[0]].1 != flat[w[1]].1 {
            let mut x: Vec<i32> = a.notes.iter().map(|n| n[0]).collect();
            let mut y: Vec<i32> = b.notes.iter().map(|n| n[0]).collect();
            x.sort();
            y.sort();
            if x.iter().zip(&y).all(|(p, q)| (p - q).abs() <= 2) && x != y {
                out.push((7, flat[w[0]].0, flat[w[1]].0));
            }
        }
    }
    out.sort_by_key(|m| (m.0, m.1));
    let mut merged: Vec<(usize, Spot, Spot)> = Vec::new();
    for m in out {
        match merged.last_mut() {
            Some(l) if l.0 == m.0 && m.1 <= l.2 => l.2 = l.2.max(m.2),
            _ => merged.push(m),
        }
    }
    let mut out = merged;
    out.sort_by_key(|m| (m.1, m.0));
    out.dedup();
    out
}

/// The beats from one spot to another (inclusive), with their starts counted in quarter notes from the first.
fn span(t: &Tune, from: Spot, to: Spot) -> Vec<Beat> {
    let mut out = Vec::new();
    let mut bar_start = 0.0f32;
    let mut first: Option<f32> = None;
    for (bi, b) in t.bars.iter().enumerate() {
        let bar_q = b.time.0 as f32 * 4.0 / b.time.1.max(1) as f32;
        for (ki, k) in b.beats.iter().enumerate() {
            if (bi, ki) >= from && (bi, ki) <= to {
                let at = bar_start + k.at;
                let f = *first.get_or_insert(at);
                let mut c = k.clone();
                c.at = at - f;
                out.push(c);
            }
        }
        bar_start += bar_q;
    }
    out
}

/// The grips a tune draws for the chords in these beats.
fn grips_of(t: &Tune, beats: &[Beat]) -> Vec<transcription::Grip> {
    let mut out: Vec<transcription::Grip> = Vec::new();
    for b in beats {
        if let Some(c) = &b.chord {
            if let Some(g) = t.grips.iter().find(|g| &g.name == c) {
                if !out.iter().any(|o| o.name == g.name) {
                    out.push(g.clone());
                }
            }
        }
    }
    out
}

/// A move's beats as bars of four (to draw them as the songs are).
fn as_bars(beats: &[Beat]) -> Vec<transcription::Bar> {
    let mut bars: Vec<transcription::Bar> = Vec::new();
    for b in beats {
        let n = (b.at / 4.0).floor().max(0.0) as usize;
        while bars.len() <= n {
            bars.push(transcription::Bar { time: (4, 4), key: None, beats: Vec::new() });
        }
        let mut c = b.clone();
        c.at -= n as f32 * 4.0;
        bars[n].beats.push(c);
    }
    bars.retain(|b| !b.beats.is_empty());
    bars
}

/// A beat's notes moved by `shift` semitones, kept on the same strings (an octave down or up if they'd fall off).
fn shifted(b: &Beat, shift: i32) -> Beat {
    let mut c = b.clone();
    if shift == 0 {
        return c;
    }
    let lo = c.notes.iter().map(|n| n[2] + shift).min().unwrap_or(0);
    let hi = c.notes.iter().map(|n| n[2] + shift).max().unwrap_or(0);
    let adj = if lo < 0 {
        12
    } else if hi > 22 {
        -12
    } else {
        0
    };
    for n in c.notes.iter_mut() {
        n[0] += shift + adj;
        n[2] += shift + adj;
    }
    if let Some(ch) = c.chord.as_mut() {
        *ch = crate::chart::transpose(ch, shift);
    }
    c
}

/// The height left on screen under the cursor (CHORD MELODIES scrolls itself, so its play row stays put).
fn room(ui: &egui::Ui) -> f32 {
    (ui.clip_rect().max.y - ui.cursor().min.y - 4.0).max(120.0)
}

/// What the pointer is on in the bars: a beat, a move's label, a move's band.
#[derive(Default)]
pub(crate) struct BarsOut {
    hover: Option<Spot>,
    grip: Option<Spot>,
    chip: Option<usize>,
    band: Option<usize>,
}

/// A note head as written: its staff step (E above middle C's line = 30), accidental, pitch, tied from the one
/// before, pushed aside (a second: -1 left, 1 right), and its accidental's column (0 none).
struct Head {
    step: i32,
    acc: i32,
    /// the accidental written before it (Some(0) a natural), if any
    mark: Option<i32>,
    midi: i32,
    tied: bool,
    side: i8,
    acc_col: u8,
}

/// One thing written in a bar: a beat's notes (or a rest), its value (in quarters, 4 = whole) and dots, a triplet,
/// flags, stem up, its beam group, and the room it needs left and right of its note heads.
struct Item {
    at: f32,
    len: f32,
    beat: Option<usize>,
    first: bool,
    heads: Vec<Head>,
    base: f32,
    dots: u8,
    trip: bool,
    flags: u8,
    up: bool,
    group: Option<usize>,
    left: f32,
    right: f32,
}

const BASES: [f32; 6] = [4.0, 2.0, 1.0, 0.5, 0.25, 0.125];

/// A length (in quarters) as a written value: the plain value, its dots, a triplet.
fn note_value(len: f32) -> (f32, u8, bool) {
    let near = |a: f32, b: f32| (a - b).abs() < 0.02;
    if let Some(b) = BASES.iter().find(|b| near(len, **b)) {
        return (*b, 0, false);
    }
    for b in BASES {
        if near(len, b * 1.5) {
            return (b, 1, false);
        }
        if near(len, b * 1.75) {
            return (b, 2, false);
        }
        if near(len, b * 2.0 / 3.0) {
            return (b, 0, true);
        }
    }
    (BASES.iter().copied().find(|b| *b <= len + 0.01).unwrap_or(0.125), 0, false)
}

fn flags_of(base: f32) -> u8 {
    if base >= 1.0 {
        0
    } else if base >= 0.5 {
        1
    } else if base >= 0.25 {
        2
    } else {
        3
    }
}

/// A bar as written: its beats' notes (seconds side by side, accidentals in columns), the rests between them,
/// stems up below the middle line and down above it, eighths and shorter beamed a beat at a time.
/// What the key signature does to a letter (0 = C .. 6 = B): +1, -1 or 0.
fn key_acc(key: i32, letter: i32) -> i32 {
    const SHARPS: [i32; 7] = [3, 0, 4, 1, 5, 2, 6];
    const FLATS: [i32; 7] = [6, 2, 5, 1, 4, 0, 3];
    if key > 0 && SHARPS[..key.min(7) as usize].contains(&letter) {
        1
    } else if key < 0 && FLATS[..(-key).min(7) as usize].contains(&letter) {
        -1
    } else {
        0
    }
}

/// A key signature's width.
fn key_w(key: i32, g: f32) -> f32 {
    if key == 0 {
        0.0
    } else {
        key.abs() as f32 * 0.85 * g + 0.6 * g
    }
}

/// A key signature on the treble staff, from `x`.
fn key_sig(p: &egui::Painter, x: f32, staff_bot: f32, key: i32, g: f32, col: Color32) {
    const SHARP_STEPS: [i32; 7] = [38, 35, 39, 36, 33, 37, 34];
    const FLAT_STEPS: [i32; 7] = [34, 37, 33, 36, 32, 35, 31];
    let steps: &[i32] = if key > 0 { &SHARP_STEPS[..key.min(7) as usize] } else { &FLAT_STEPS[..(-key).min(7) as usize] };
    for (i, s) in steps.iter().enumerate() {
        let y = staff_bot - (s - 30) as f32 * g / 2.0;
        crate::lines_ui::accidental(p, Pos2::new(x + 0.6 * g + i as f32 * 0.85 * g, y), key.signum(), g, col);
    }
}

fn bar_items(bar: &transcription::Bar, sharps: bool, g: f32, tab: bool) -> Vec<Item> {
    // spelled as the key spells (with sharps or flats); with none given, every accidental written
    let key = bar.key.map_or(0, |k| k as i32);
    let sharps = if key != 0 { key > 0 } else { sharps };
    // what each staff step carries in this bar: the key's, then any accidental written before
    let mut carried: Vec<(i32, i32)> = Vec::new();
    let bar_q = bar.time.0 as f32 * 4.0 / bar.time.1.max(1) as f32;
    let hw = 0.62 * g;
    let mut items: Vec<Item> = Vec::new();
    let rest = |items: &mut Vec<Item>, at: f32, len: f32, beat: Option<usize>| {
        let (mut a, mut left) = (at, len);
        let mut first = true;
        while left > 0.06 {
            let b = BASES.iter().copied().find(|b| *b <= left + 0.01).unwrap_or(0.125);
            items.push(Item {
                at: a,
                len: b,
                beat,
                first,
                heads: Vec::new(),
                base: b,
                dots: 0,
                trip: false,
                flags: 0,
                up: true,
                group: None,
                left: 0.8 * g,
                right: 0.8 * g,
            });
            first = false;
            a += b;
            left -= b;
        }
    };
    let mut cursor = 0.0f32;
    for (k, beat) in bar.beats.iter().enumerate() {
        if beat.at > cursor + 0.06 {
            rest(&mut items, cursor, beat.at - cursor, None);
        }
        if beat.notes.is_empty() {
            rest(&mut items, beat.at, beat.len.max(0.125), Some(k));
        } else {
            let mut heads: Vec<Head> = beat
                .notes
                .iter()
                .map(|n| {
                    let (letter, acc, oct) = lines::spell(n[0] + 12, sharps);
                    Head { step: oct * 7 + letter, acc, mark: None, midi: n[0], tied: n[3] == 1, side: 0, acc_col: 0 }
                })
                .collect();
            heads.sort_by_key(|h| (h.step, h.acc));
            heads.dedup_by_key(|h| (h.step, h.acc));
            for h in heads.iter_mut() {
                let now = carried.iter().find(|c| c.0 == h.step).map_or(key_acc(key, h.step.rem_euclid(7)), |c| c.1);
                if h.acc != now && !h.tied {
                    h.mark = Some(h.acc);
                }
                match carried.iter_mut().find(|c| c.0 == h.step) {
                    Some(c) => c.1 = h.acc,
                    None => carried.push((h.step, h.acc)),
                }
            }
            let (base, dots, trip) = note_value(beat.len);
            let mid = heads[0].step + heads[heads.len() - 1].step;
            let tab_half = if tab {
                beat.notes.iter().map(|n| crate::font::text_w(&n[2].to_string(), 1.0) / 2.0 + 3.0).fold(0.0, f32::max)
            } else {
                0.0
            };
            items.push(Item {
                at: beat.at,
                len: beat.len,
                beat: Some(k),
                first: true,
                heads,
                base,
                dots,
                trip,
                flags: flags_of(base),
                up: mid < 68,
                group: None,
                left: tab_half,
                right: tab_half,
            });
        }
        cursor = cursor.max(beat.at + beat.len);
    }
    if cursor < bar_q - 0.06 {
        rest(&mut items, cursor, bar_q - cursor, None);
    }
    // a bar of nothing: one whole rest
    if items.iter().all(|i| i.heads.is_empty()) {
        let beat = items.first().and_then(|i| i.beat);
        items = vec![Item {
            at: 0.0,
            len: bar_q,
            beat,
            first: true,
            heads: Vec::new(),
            base: 4.0,
            dots: 0,
            trip: false,
            flags: 0,
            up: true,
            group: None,
            left: g,
            right: g,
        }];
    }
    // beams: eighths and shorter, together within a beat (a dotted quarter in 6/8, 9/8, 12/8)
    let window = if bar.time.1 == 8 && bar.time.0 % 3 == 0 { 1.5 } else { 1.0 };
    let mut gid = 0;
    let mut i = 0;
    while i < items.len() {
        let key = |it: &Item| ((it.at + 1e-3) / window).floor() as i32;
        let can = |it: &Item| !it.heads.is_empty() && it.flags > 0;
        if !can(&items[i]) {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < items.len() && can(&items[j]) && key(&items[j]) == key(&items[i]) {
            j += 1;
        }
        if j - i >= 2 {
            // the stems all one way: by where the group sits on the staff
            let sum: i32 = items[i..j].iter().map(|it| it.heads[0].step + it.heads[it.heads.len() - 1].step - 68).sum();
            for it in items[i..j].iter_mut() {
                it.group = Some(gid);
                it.up = sum < 0;
            }
            gid += 1;
        }
        i = j;
    }
    // seconds side by side, accidentals in columns, and the room each needs
    for it in items.iter_mut().filter(|it| !it.heads.is_empty()) {
        let n = it.heads.len();
        if it.up {
            for i in 1..n {
                if it.heads[i].step - it.heads[i - 1].step == 1 && it.heads[i - 1].side == 0 {
                    it.heads[i].side = 1;
                }
            }
        } else {
            for i in (0..n.saturating_sub(1)).rev() {
                if it.heads[i + 1].step - it.heads[i].step == 1 && it.heads[i + 1].side == 0 {
                    it.heads[i].side = -1;
                }
            }
        }
        let mut cols: Vec<Vec<i32>> = Vec::new();
        for i in (0..n).rev() {
            if it.heads[i].mark.is_some() {
                let s = it.heads[i].step;
                let c = match cols.iter().position(|c| c.iter().all(|x| (x - s).abs() >= 6)) {
                    Some(c) => c,
                    None => {
                        cols.push(Vec::new());
                        cols.len() - 1
                    }
                };
                cols[c].push(s);
                it.heads[i].acc_col = c as u8 + 1;
            }
        }
        let lside = if it.heads.iter().any(|h| h.side < 0) { 2.0 * hw } else { 0.0 };
        let rside = if it.heads.iter().any(|h| h.side > 0) { 2.0 * hw } else { 0.0 };
        let flag = if it.group.is_none() && it.up && it.flags > 0 { 1.0 * g } else { 0.0 };
        it.left = it.left.max(hw * 1.6 + lside + cols.len() as f32 * 1.1 * g);
        it.right = it.right.max(hw * 1.6 + rside + it.dots as f32 * 0.6 * g + flag);
    }
    items
}

/// A rest, centred on the staff's middle line.
fn rest(p: &egui::Painter, c: Pos2, base: f32, g: f32, col: Color32) {
    let pt = |x: f32, y: f32| Pos2::new(c.x + x * g, c.y + y * g);
    if base >= 4.0 {
        p.rect_filled(Rect::from_min_max(pt(-0.6, -1.0), pt(0.6, -0.5)), 0.0, col);
    } else if base >= 2.0 {
        p.rect_filled(Rect::from_min_max(pt(-0.6, -0.5), pt(0.6, 0.0)), 0.0, col);
    } else if base >= 1.0 {
        p.add(egui::Shape::line(
            vec![pt(-0.2, -1.5), pt(0.35, -0.8), pt(-0.25, -0.15), pt(0.3, 0.5), pt(-0.15, 0.35), pt(-0.3, 0.8), pt(0.1, 1.3)],
            Stroke::new((0.28 * g).max(1.5), col),
        ));
    } else {
        let n = flags_of(base);
        p.line_segment([pt(0.45, -0.9), pt(-0.05, 0.9 + (n as f32 - 1.0) * 0.8)], Stroke::new(1.3_f32, col));
        for i in 0..n {
            let y = -0.7 + i as f32 * 0.8;
            p.circle_filled(pt(-0.3 - i as f32 * 0.1, y), 0.27 * g, col);
            p.line_segment([pt(-0.3 - i as f32 * 0.1, y + 0.1), pt(0.45 - i as f32 * 0.25, y - 0.1)], Stroke::new(1.2_f32, col));
        }
    }
}

/// The neck for a move: the grip (named by degree, the root square), the notes within the hand's reach of it
/// that fit the chord (rings), and the move's own notes in its colour (for a grip move or a melody from the grip,
/// only the ones that aren't the grip).
fn focus_dots(grip: &[Option<u8>; 6], chord: Option<&str>, beats: &[Beat], kind: Option<usize>) -> Vec<crate::improv::NeckDot> {
    const OPEN: [i32; 6] = [40, 45, 50, 55, 59, 64];
    let mut dots: Vec<crate::improv::NeckDot> = Vec::new();
    let notes = chord.and_then(theory::chord_notes);
    let root = notes.as_ref().map(|n| n.root.rem_euclid(12));
    let name = |m: i32| root.map_or(String::new(), |r| crate::fretboard::degree(m.rem_euclid(12), r).0.to_string());
    let add = |dots: &mut Vec<crate::improv::NeckDot>, s: usize, f: u8, col: Color32, strength: f32, ring: bool| {
        if let Some(d) = dots.iter_mut().find(|d| d.spot == (s, f)) {
            if strength >= d.strength {
                d.col = col;
                d.strength = strength;
                d.ring = ring;
            }
            return;
        }
        let m = OPEN[s] + f as i32;
        dots.push(crate::improv::NeckDot {
            spot: (s, f),
            text: name(m),
            col,
            strength,
            ring,
            root: root == Some(m.rem_euclid(12)) && !ring,
        });
    };
    // in reach: the grip's frets, one below, two above
    let fretted: Vec<u8> = grip.iter().flatten().copied().filter(|f| *f > 0).collect();
    if let (Some(lo), Some(hi), Some(n)) = (fretted.iter().min(), fretted.iter().max(), &notes) {
        let r = n.root.rem_euclid(12);
        let minor = n.pcs.iter().any(|p| (p - r).rem_euclid(12) == 3) && !n.pcs.iter().any(|p| (p - r).rem_euclid(12) == 4);
        let mut fits: Vec<i32> = n.pcs.iter().map(|p| p.rem_euclid(12)).collect();
        fits.push((r + 2) % 12);
        fits.push((r + if minor { 5 } else { 9 }) % 12);
        for s in 0..6 {
            for f in lo.saturating_sub(1)..=(hi + 2).min(24) {
                if fits.contains(&((OPEN[s] + f as i32).rem_euclid(12))) {
                    add(&mut dots, s, f, pal().ink2, 0.55, true);
                }
            }
        }
    }
    for (s, f) in grip.iter().enumerate() {
        if let Some(f) = f {
            add(&mut dots, s, *f, pal().ink, 0.9, false);
        }
    }
    let col = kind.map_or(pal().red, |k| {
        let (r, g, b) = MARK_COLS[k.min(9)];
        Color32::from_rgb(r, g, b)
    });
    let only_moving = matches!(kind, Some(6) | Some(9));
    for bt in beats {
        for n in bt.notes.iter().filter(|n| n[3] == 0 && (0..6).contains(&n[1])) {
            let (s, f) = (n[1] as usize, n[2].clamp(0, 24) as u8);
            if only_moving && grip[s] == Some(f) {
                continue;
            }
            add(&mut dots, s, f, col, 1.0, false);
        }
    }
    dots
}

/// One step of a move as the panel shows it: where it starts (and the next starts), its chord, what rings
/// (string, fret, pitch), the pitches new since the step before, and whether it is the chord the move lands on.
struct Step {
    at: Spot,
    until: Option<Spot>,
    chord: Option<String>,
    notes: Vec<(i32, i32, i32)>,
    moved: Vec<i32>,
    target: bool,
}

/// A move step by step: what rings at each beat from `from` to `to` (struck, or still held - the latest on each
/// string) whenever it changes, then the next chord it lands on. At most ten.
fn move_steps(t: &Tune, from: Spot, to: Spot, first_chord: Option<String>) -> Vec<Step> {
    let mut flat: Vec<(Spot, f32, f32, &Beat)> = Vec::new();
    let mut q = 0.0f32;
    for (bi, b) in t.bars.iter().enumerate() {
        for (k, bt) in b.beats.iter().enumerate() {
            flat.push(((bi, k), q + bt.at, q + bt.at + bt.len, bt));
        }
        q += b.time.0 as f32 * 4.0 / b.time.1.max(1) as f32;
    }
    let ring = |i: usize| -> Vec<(i32, i32, i32)> {
        let s = flat[i].1;
        let mut v: Vec<(i32, i32, i32)> = Vec::new();
        for j in (i.saturating_sub(16)..=i).rev() {
            if j != i && flat[j].2 <= s + 0.01 {
                continue;
            }
            for n in &flat[j].3.notes {
                if (0..6).contains(&n[1]) && !v.iter().any(|x| x.0 == n[1]) {
                    v.push((n[1], n[2], n[0]));
                }
            }
        }
        v.sort();
        v
    };
    let Some(a) = flat.iter().position(|f| f.0 >= from) else { return Vec::new() };
    let z = flat.iter().rposition(|f| f.0 <= to).unwrap_or(a).max(a);
    let mut steps: Vec<Step> = Vec::new();
    let mut chord = first_chord;
    for i in a..=z {
        if let Some(c) = &flat[i].3.chord {
            chord = Some(c.clone());
        }
        let r = ring(i);
        if r.len() < 2 || steps.last().is_some_and(|l| l.notes == r && l.chord == chord) {
            continue;
        }
        if steps.len() >= 10 {
            break;
        }
        let moved: Vec<i32> = match steps.last() {
            Some(l) => r.iter().filter(|n| !l.notes.iter().any(|m| m.0 == n.0 && m.1 == n.1)).map(|n| n.2).collect(),
            None => Vec::new(),
        };
        if let Some(l) = steps.last_mut() {
            l.until = Some(flat[i].0);
        }
        steps.push(Step { at: flat[i].0, until: None, chord: chord.clone(), notes: r, moved, target: false });
    }
    // the chord it lands on
    let last_chord = steps.last().and_then(|s| s.chord.clone());
    if let Some(i) = (z + 1..(z + 17).min(flat.len())).find(|i| flat[*i].3.chord.is_some() && flat[*i].3.chord != last_chord) {
        let r = ring(i);
        if r.len() >= 2 {
            let moved: Vec<i32> = match steps.last() {
                Some(l) => r.iter().filter(|n| !l.notes.iter().any(|m| m.0 == n.0 && m.1 == n.1)).map(|n| n.2).collect(),
                None => Vec::new(),
            };
            if let Some(l) = steps.last_mut() {
                l.until = Some(flat[i].0);
            }
            steps.push(Step { at: flat[i].0, until: None, chord: flat[i].3.chord.clone(), notes: r, moved, target: true });
        }
    }
    steps
}

/// A chord name's root (and bass) spelled with sharps or flats.
fn respell(name: &str, sharps: bool) -> String {
    let (main, bass) = match name.split_once('/') {
        Some((m, b)) => (m, Some(b)),
        None => (name, None),
    };
    let (r, rest) = crate::chart::split_root(main);
    let Some(pc) = crate::chart::note_index(r) else { return name.to_string() };
    let mut out = format!("{}{}", theory::note_name(pc, sharps), rest);
    if let Some(b) = bass {
        match crate::chart::note_index(b) {
            Some(bp) => out = format!("{}/{}", out, theory::note_name(bp, sharps)),
            None => out = format!("{}/{}", out, b),
        }
    }
    out
}

/// When a transcription came without chord names: name the chords from the notes (each beat of three notes or
/// more where the harmony changes), with grips from the frets played. Says whether it did.
fn name_from_notes(t: &mut Tune) -> bool {
    if t.bars.iter().any(|b| b.beats.iter().any(|k| k.chord.is_some())) {
        return false;
    }
    let mut last = String::new();
    for b in t.bars.iter_mut() {
        for k in b.beats.iter_mut() {
            if k.notes.len() < 3 || k.notes.iter().filter(|n| n[3] == 0).count() < 2 {
                continue;
            }
            let mut ms: Vec<i32> = k.notes.iter().map(|n| n[0]).collect();
            ms.sort();
            let lo = ms[0];
            // with the melody on top, and without it: the simpler name (the root in the bass, then the shorter)
            let mut names: Vec<String> = theory::name_chords(&ms, Some(lo)).into_iter().take(1).collect();
            if ms.len() >= 4 {
                names.extend(theory::name_chords(&ms[..ms.len() - 1], Some(lo)).into_iter().take(1));
            }
            let Some(name) = names.into_iter().min_by_key(|n| (n.contains('/'), n.len())) else { continue };
            // spelled as the key is (F#m9, not Gbm9, in A)
            let name = if b.key.is_some_and(|k| k > 0) { respell(&name, true) } else { name };
            if name == last {
                continue;
            }
            last = name.clone();
            if !t.grips.iter().any(|g| g.name == name) {
                let mut frets = [-1i8; 6];
                for n in &k.notes {
                    if (0..6).contains(&n[1]) {
                        frets[n[1] as usize] = n[2].clamp(0, 24) as i8;
                    }
                }
                t.grips.push(transcription::Grip { name: name.clone(), frets });
            }
            k.chord = Some(name);
        }
    }
    true
}

impl App {
    /// CHORD MELODIES: SONGS and MOVES.
    pub(crate) fn melodies_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        ui.add_space(6.0);
        if let Some(t) = tab_row(ui, &["SONGS", "MOVES"], self.cm_tab as usize) {
            self.cm_tab = t as u8;
        }
        ui.add_space(6.0);
        // the tunes in the folder (read once, again after an import)
        if self.cm_list.is_none() {
            self.cm_list = Some(transcription::list());
        }
        if !self.cm_moves_read {
            self.cm_moves = transcription::load_moves();
            self.cm_hidden = transcription::load_hidden();
            self.cm_moves_read = true;
        }
        if self.cm_tab == 0 {
            self.cm_songs(ui, acts);
        } else {
            egui::ScrollArea::vertical()
                .id_salt("cm_moves")
                .auto_shrink([false, false])
                .max_height(room(ui))
                .show(ui, |ui| self.cm_moves_view(ui, acts));
        }
    }

    /// The tune open (read from its file the first time).
    fn cm_tune(&mut self) -> Option<std::sync::Arc<Tune>> {
        let path = self.cm_file.clone()?;
        if self.cm_loaded.as_ref().map(|l| &l.0) != Some(&path) {
            match transcription::load(&path) {
                Ok(mut t) => {
                    t.grips.retain(|g| g.frets.iter().any(|f| *f >= 0));
                    self.cm_named = name_from_notes(&mut t);
                    self.cm_bpm = if t.tempo > 0.0 { t.tempo.round() as u32 } else { 90 };
                    self.cm_loaded = Some((path.clone(), std::sync::Arc::new(t)));
                }
                Err(e) => {
                    self.set_err(format!("CAN'T OPEN THAT TRANSCRIPTION: {}", e));
                    self.cm_file = None;
                    return None;
                }
            }
        }
        // (shared, not copied: a tune is hundreds of bars)
        self.cm_loaded.as_ref().map(|l| l.1.clone())
    }

    fn cm_songs(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let list = self.cm_list.clone().unwrap_or_default();
        if self.cm_file.is_none() {
            self.cm_file = list.first().map(|l| l.0.clone());
        }
        let tune = if list.is_empty() { None } else { self.cm_tune() };
        let Some(tune) = tune else {
            egui::ScrollArea::vertical()
                .id_salt("cm_songs")
                .auto_shrink([false, false])
                .max_height(room(ui))
                .show(ui, |ui| self.cm_top(ui, acts, &list));
            return;
        };
        let nb = tune.bars.len();
        self.cm_to = self.cm_to.min(nb - 1);
        self.cm_from = self.cm_from.min(self.cm_to);
        // sharps or flats: as the key signature has it, else as the first chord's key would
        let sharps = match tune.bars.first().and_then(|b| b.key).filter(|k| *k != 0) {
            Some(k) => k > 0,
            None => tune
                .bars
                .iter()
                .flat_map(|b| b.beats.iter())
                .find_map(|b| b.chord.as_ref().and_then(|c| chord_root(c)))
                .map_or(false, lines::key_sharps),
        };
        // the moves Backline spots (worked out once for each tune), marked over the bars
        let path = self.cm_file.clone().unwrap_or_default();
        if self.cm_found.as_ref().map(|f| &f.0) != Some(&path) {
            self.cm_found = Some((path.clone(), suggest(&tune)));
            self.cm_mark = None;
        }
        let marks: Vec<(usize, Spot, Spot)> = if self.cm_marks {
            let mut all = self.cm_found.as_ref().map(|f| f.1.clone()).unwrap_or_default();
            for m in self.cm_moves.iter().filter(|m| m.tune == tune.title) {
                if let Some((a, z)) = m.spots {
                    if !all.contains(&(m.kind, a, z)) {
                        all.push((m.kind, a, z));
                    }
                }
            }
            // not the ones you removed
            all.retain(|m| !self.cm_hidden.iter().any(|h| h.0 == tune.title && h.1 == m.0 && h.2 == m.1 && h.3 == m.2));
            all.sort_by_key(|m| (m.1, m.0));
            all.into_iter().filter(|m| self.cm_mark_kind == 0 || m.0 == self.cm_mark_kind as usize - 1).collect()
        } else {
            Vec::new()
        };
        // (a test switch: BACKLINE_CM_PICK=<kind> opens the first move of that kind in the panel, once)
        static PICKED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if let Some(k) = std::env::var("BACKLINE_CM_PICK").ok().and_then(|v| v.parse::<usize>().ok()) {
            if !PICKED.swap(true, std::sync::atomic::Ordering::Relaxed) {
                if let Some(i) = marks.iter().position(|m| m.0 == k) {
                    self.cm_mark = Some(i);
                    self.cm_sel = Some(marks[i].1);
                    self.cm_sel2 = Some(marks[i].2);
                }
            }
        }
        // the beat sounding now
        let now = self.cm_now(&tune);
        // ---- pinned at the top: playing, and the beats picked (a loop, a move) up close
        ui.horizontal_wrapped(|ui| {
            crate::views::label(ui, "PLAY", 100.0);
            let playing = self.cm_play.is_some();
            if retro_btn(ui, if playing { "STOP" } else { "PLAY" }, playing).clicked() {
                if playing {
                    self.player.send(Cmd::StopOnce);
                    self.cm_play = None;
                } else {
                    self.cm_start(&tune);
                }
            }
            if retro_btn(ui, "LOOP", self.cm_loop).tip("Round and round - drag across the bars below to pick which").clicked() {
                self.cm_loop = !self.cm_loop;
            }
            if retro_btn(ui, "WHOLE SONG", self.cm_from == 0 && self.cm_to + 1 >= nb)
                .tip("Play (or loop) the whole song")
                .clicked()
            {
                self.cm_from = 0;
                self.cm_to = nb - 1;
                self.cm_mark = None;
            }
            crate::views::label(ui, "BARS", 50.0);
            let bars: Vec<String> = (1..=nb).map(|b| b.to_string()).collect();
            let bars: Vec<&str> = bars.iter().map(String::as_str).collect();
            if let Some(i) = crate::dropdown_fixed(ui, "cm_from", &bars, self.cm_from, 70.0) {
                self.cm_from = i;
                self.cm_to = self.cm_to.max(i);
            }
            crate::views::label(ui, "TO", 30.0);
            if let Some(i) = crate::dropdown_fixed(ui, "cm_to", &bars, self.cm_to, 70.0) {
                self.cm_to = i;
                self.cm_from = self.cm_from.min(i);
            }
            crate::views::label(ui, "TEMPO", 60.0);
            if retro_btn_w(ui, "-", 28.0, false).clicked() {
                self.cm_bpm = self.cm_bpm.saturating_sub(5).max(20);
            }
            lcd_box(ui, &format!("{}", self.cm_bpm), 50.0, pal().ink);
            if retro_btn_w(ui, "+", 28.0, false).clicked() {
                self.cm_bpm = (self.cm_bpm + 5).min(300);
            }
        });
        self.cm_focus(ui, acts, &tune, sharps, now, &marks);
        ui.add_space(4.0);
        // ---- below, scrolling: the tune, what is shown, and the bars
        let mut area = egui::ScrollArea::vertical().id_salt("cm_songs").auto_shrink([false, false]).max_height(room(ui));
        // (a test switch: BACKLINE_CM_SCROLL opens the bars scrolled down that far, once)
        static SCROLLED: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        if let Some(v) = std::env::var("BACKLINE_CM_SCROLL").ok().and_then(|v| v.parse::<f32>().ok()) {
            if SCROLLED.fetch_add(1, std::sync::atomic::Ordering::Relaxed) < 60 {
                area = area.vertical_scroll_offset(v);
            }
        }
        area.show(ui, |ui| {
            self.cm_top(ui, acts, &list);
            if self.cm_named {
                crate::views::dim_line(ui, "THIS FILE HAS NO CHORD NAMES - THESE ARE NAMED FROM THE NOTES", 1.0, pal().dim);
            }
            crate::views::dim_line(
                ui,
                "DRAG ACROSS THE BARS TO LOOP THEM - CLICK A BEAT TO SEE IT - CLICK A COLOURED MOVE TO LOOP IT",
                1.0,
                pal().dim,
            );
            ui.add_space(6.0);
            let out = self.draw_bars(ui, &tune.bars, &tune.grips, sharps, now, true, &marks, 0);
            // press on a beat and drag to another: those bars looped (as in iReal Pro); a click picks a beat, or a move
            let (pressed, down, released, shift) = ui.input(|i| {
                (i.pointer.primary_pressed(), i.pointer.primary_down(), i.pointer.primary_released(), i.modifiers.shift)
            });
            if pressed {
                self.cm_drag = out.grip.or(out.hover).or(out.chip.map(|i| marks[i].1)).map(|h| (h, h));
            }
            if down {
                if let (Some(d), Some(h)) = (self.cm_drag.as_mut(), out.hover) {
                    d.1 = h;
                }
            }
            if released {
                if let Some((a, z)) = self.cm_drag.take() {
                    let mut relooped = false;
                    if a == z && out.chip.is_none() && out.grip == Some(a) {
                        self.cm_mark = None;
                        self.cm_sel = Some(a);
                        self.cm_sel2 = None;
                    } else if a == z {
                        match out.chip.or(out.band) {
                            Some(i) => {
                                let m = marks[i];
                                self.cm_mark = Some(i);
                                self.cm_sel = Some(m.1);
                                self.cm_sel2 = Some(m.2);
                                self.cm_from = m.1 .0;
                                self.cm_to = m.2 .0;
                                self.cm_loop = true;
                                relooped = true;
                            }
                            None => {
                                self.cm_mark = None;
                                if shift && self.cm_sel.is_some() {
                                    self.cm_sel2 = Some(a);
                                } else {
                                    self.cm_sel = Some(a);
                                    self.cm_sel2 = None;
                                }
                            }
                        }
                    } else {
                        let (lo, hi) = if a <= z { (a, z) } else { (z, a) };
                        self.cm_mark = None;
                        self.cm_sel = Some(lo);
                        self.cm_sel2 = Some(hi);
                        self.cm_from = lo.0;
                        self.cm_to = hi.0;
                        self.cm_loop = true;
                        relooped = true;
                    }
                    if relooped && self.cm_play.is_some() {
                        self.cm_start(&tune);
                    }
                }
            }
        });
    }

    /// The top of SONGS: bringing tunes in, the tune, what is shown.
    fn cm_top(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, list: &[(std::path::PathBuf, String)]) {
        para(ui, "Your transcriptions, bar by bar: the chord names and grips, the notes on a staff with the tab and their names. Play them at any speed, loop a few bars, click a beat to see what it is, and keep the moves you like.", pal().ink2);
        ui.horizontal_wrapped(|ui| {
            if retro_btn(ui, "IMPORT GUITAR PRO...", false)
                .tip("Turn Guitar Pro files (.gp) into transcriptions Backline keeps - the originals aren't needed after")
                .clicked()
            {
                acts.push(Action::CmImport(false));
            }
            if retro_btn(ui, "IMPORT A TRANSCRIPTION...", false)
                .tip("A Backline transcription file (.json) someone sent you")
                .clicked()
            {
                acts.push(Action::CmImport(true));
            }
            if retro_btn(ui, "OPEN FOLDER", false)
                .tip("Where your transcriptions are kept - copy one from here to send it")
                .clicked()
            {
                let d = transcription::dir();
                let _ = std::fs::create_dir_all(&d);
                let _ = webbrowser::open(&d.to_string_lossy());
            }
        });
        if list.is_empty() {
            para(ui, "No transcriptions yet. IMPORT GUITAR PRO... turns .gp files into ones Backline keeps.", pal().ink2);
            return;
        }
        let names: Vec<&str> = list.iter().map(|l| l.1.as_str()).collect();
        let cur = self.cm_file.as_ref().and_then(|f| list.iter().position(|l| &l.0 == f)).unwrap_or(0);
        if let Some(i) = crate::choice_line(ui, "cm_tune", "TUNE", &names, cur, 360.0) {
            self.cm_file = Some(list[i].0.clone());
            self.cm_sel = None;
            self.cm_sel2 = None;
            self.cm_from = 0;
            self.cm_to = usize::MAX;
            self.cm_mark = None;
        }
        // what is shown: all of it unless you leave some out (leave the chord names out to name them yourself first)
        ui.horizontal_wrapped(|ui| {
            crate::views::label(ui, "SHOW", 100.0);
            for (i, n) in ["CHORD NAMES", "GRIPS", "STAFF", "TAB", "NOTE NAMES"].iter().enumerate() {
                if crate::check_box(ui, n, self.cm_show[i]).clicked() {
                    self.cm_show[i] = !self.cm_show[i];
                }
            }
            if crate::check_box(ui, "MOVES SPOTTED", self.cm_marks)
                .tip("A soft colour over the beats where Backline spots a move - click one to loop it")
                .clicked()
            {
                self.cm_marks = !self.cm_marks;
                self.cm_mark = None;
            }
        });
        if self.cm_marks {
            let mut kinds: Vec<&str> = vec!["ALL KINDS"];
            kinds.extend(KINDS.iter());
            if let Some(i) = crate::choice_line(ui, "cm_mark_kind", "MOVES", &kinds, self.cm_mark_kind as usize, 360.0) {
                self.cm_mark_kind = i as u8;
                self.cm_mark = None;
            }
            // the ones you removed from this tune, back again
            let title = self.cm_loaded.as_ref().map(|l| l.1.title.clone()).unwrap_or_default();
            let gone = self.cm_hidden.iter().filter(|h| h.0 == title).count();
            if gone > 0
                && retro_btn(ui, &format!("BRING BACK {} REMOVED", gone), false)
                    .tip("The moves you removed from this tune")
                    .clicked()
            {
                self.cm_hidden.retain(|h| h.0 != title);
                transcription::save_hidden(&self.cm_hidden);
                self.cm_mark = None;
            }
        }
    }

    /// The beats picked - a loop dragged, a move clicked, a beat - looked at in a small panel pinned under the play
    /// row: play, keep or close it; the chord's grip; the neck with the grip, the notes in reach that fit the chord,
    /// and the move's own notes lit; what the move is and what each chord's notes are.
    fn cm_focus(
        &mut self,
        ui: &mut egui::Ui,
        acts: &mut Vec<Action>,
        tune: &Tune,
        sharps: bool,
        now: Option<Spot>,
        marks: &[(usize, Spot, Spot)],
    ) {
        let Some(a) = self.cm_sel else { return };
        let b = self.cm_sel2.unwrap_or(a);
        let (from, to) = if a <= b { (a, b) } else { (b, a) };
        let to = (to.0.min(tune.bars.len() - 1), to.1);
        let picked_mark = self.cm_mark.and_then(|i| marks.get(i)).copied();
        let sc = crate::font::ui_scale();
        // the chord at the first beat (the last one named), its grip
        let chord = (0..=from.0).rev().find_map(|bi| {
            let beats = &tune.bars[bi].beats;
            let upto = if bi == from.0 { from.1.min(beats.len().saturating_sub(1)) } else { beats.len().saturating_sub(1) };
            (0..beats.len()).rev().filter(|k| *k <= upto).find_map(|k| beats[k].chord.clone())
        });
        let grip: [Option<u8>; 6] = match chord.as_ref().and_then(|c| tune.grips.iter().find(|g| &g.name == c)) {
            Some(g) => g.frets.map(|f| if f < 0 { None } else { Some(f as u8) }),
            None => {
                let mut s = [None; 6];
                for n in tune.bars[from.0].beats.get(from.1).map(|b| b.notes.as_slice()).unwrap_or(&[]) {
                    if (0..6).contains(&n[1]) {
                        s[n[1] as usize] = Some(n[2].clamp(0, 24) as u8);
                    }
                }
                s
            }
        };
        let beats = span(tune, from, to);
        let mut close = false;
        egui::Frame::none().fill(pal().beige).stroke(Stroke::new(1.0_f32, pal().ink2)).inner_margin(6.0).show(ui, |ui| {
            // the title, and what to do with it - always in sight
            ui.horizontal_wrapped(|ui| {
                let header = match picked_mark {
                    Some(m) => format!("{}  -  BARS {}-{}", SHORT[m.0.min(9)], from.0 + 1, to.0 + 1),
                    None if from == to => format!("BAR {}", from.0 + 1),
                    None => format!("BARS {}-{}", from.0 + 1, to.0 + 1),
                };
                crate::views::dim_line(ui, &header, 1.5, pal().ink);
                let playing = self.cm_play.is_some();
                if retro_btn(ui, if playing { "STOP" } else { "PLAY" }, playing)
                    .tip("Play these bars (round and round with LOOP)")
                    .clicked()
                {
                    if playing {
                        self.player.send(Cmd::StopOnce);
                        self.cm_play = None;
                    } else {
                        self.cm_from = from.0;
                        self.cm_to = to.0;
                        self.cm_start(tune);
                    }
                }
                if retro_btn(ui, "SAVE AS A MOVE", false).tip("Keep it in MOVES (and marked on the song)").clicked() {
                    let kind = picked_mark.map_or(MY_OWN, |m| m.0);
                    let name = match picked_mark {
                        Some(m) => format!("{} - {}", tune.title, KINDS[m.0].to_lowercase()),
                        None => format!("{} bars {}-{}", tune.title, from.0 + 1, to.0 + 1),
                    };
                    self.cm_moves.push(Move {
                        name,
                        kind,
                        tune: tune.title.clone(),
                        bars: (from.0 + 1, to.0 + 1),
                        spots: Some((from, to)),
                        grips: grips_of(tune, &beats),
                        beats: beats.clone(),
                        ..Default::default()
                    });
                    transcription::save_moves(&self.cm_moves);
                    self.cm_mark = None;
                    self.set_note("KEPT IN MOVES - MARKED ON THE SONG TOO");
                }
                if let Some(m) = picked_mark {
                    if retro_btn(ui, "REMOVE THIS MOVE", false)
                        .tip("Not one you want marked: gone from this tune (BRING BACK under MOVES returns it)")
                        .clicked()
                    {
                        // one you kept: forgotten; one Backline spotted: hidden
                        match self
                            .cm_moves
                            .iter()
                            .position(|mv| mv.tune == tune.title && mv.kind == m.0 && mv.spots == Some((m.1, m.2)))
                        {
                            Some(i) => {
                                self.cm_moves.remove(i);
                                transcription::save_moves(&self.cm_moves);
                            }
                            None => {
                                self.cm_hidden.push((tune.title.clone(), m.0, m.1, m.2));
                                transcription::save_hidden(&self.cm_hidden);
                            }
                        }
                        close = true;
                    }
                }
                if retro_btn(ui, "ANALYZER", false)
                    .tip("The first beat in the chord analyzer: every name it has, the piano")
                    .clicked()
                {
                    let mut frets = [None; 6];
                    for n in tune.bars[from.0].beats.get(from.1).map(|b| b.notes.as_slice()).unwrap_or(&[]) {
                        if (0..6).contains(&n[1]) {
                            frets[n[1] as usize] = Some(n[2].clamp(0, 24) as u8);
                        }
                    }
                    self.an_frets = frets;
                    self.chords_tab = 1;
                    acts.push(Action::Section(Sec::Chords));
                }
                if retro_btn_w(ui, "X", 28.0, false).tip("Close (the loop stays)").clicked() {
                    close = true;
                }
            });
            // the move step by step: each grip as it changes, then the chord it lands on - the notes that moved lit
            let col = picked_mark.map_or(pal().red, |m| {
                let (r, g2, bl) = MARK_COLS[m.0.min(9)];
                Color32::from_rgb(r, g2, bl)
            });
            let steps = move_steps(tune, from, to, chord.clone());
            // (its own scroll: at most about 45% of the window, so the song stays in view)
            let max_h = (ui.ctx().screen_rect().height() * 0.35).max(200.0);
            egui::ScrollArea::vertical().id_salt("cm_focus").max_height(max_h).auto_shrink([false, true]).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let bw = 78.0 * sc;
                    for (i, st) in steps.iter().enumerate() {
                        if i > 0 {
                            let (r, _) = ui.allocate_exact_size(Vec2::new(16.0 * sc, 110.0 * sc), Sense::hover());
                            ptext(ui.painter(), r.center(), Align::Center, ">", 1.5, if st.target { col } else { pal().ink2 });
                        }
                        let (r, resp) = ui.allocate_exact_size(Vec2::new(bw, 116.0 * sc), Sense::click());
                        let resp = resp.on_hover_text("Hear it, and see it on the neck");
                        if resp.clicked() {
                            self.cm_step = Some((from, i));
                            let ev: Vec<(usize, i32, f32, f32)> =
                                st.notes.iter().enumerate().map(|(j, n)| (n.0 as usize, n.2, j as f32 * 0.03, 0.85)).collect();
                            self.player.send(Cmd::StopOnce);
                            self.player.send(Cmd::Once(crate::band::string_events(&ev, 1.6), 0.9));
                        }
                        let picked_step = self.cm_step == Some((from, i));
                        let p = ui.painter();
                        if resp.hovered() || picked_step {
                            p.rect_filled(r, 3.0, pal().sel.gamma_multiply(if picked_step { 0.7 } else { 0.35 }));
                        }
                        let sounding = now.is_some_and(|n| n >= st.at && st.until.map_or(true, |u| n < u));
                        if sounding {
                            p.rect_filled(r, 3.0, pal().red.gamma_multiply(0.2));
                        }
                        if st.target {
                            p.rect_stroke(r, 3.0, Stroke::new(1.5_f32, col));
                        }
                        let title = format!(
                            "{}{}",
                            if st.target { "TO " } else { "" },
                            st.chord.as_deref().map(crate::chart::pretty_chord).unwrap_or_default()
                        );
                        ptext(
                            p,
                            Pos2::new(r.min.x + 2.0, r.min.y + 9.0 * sc),
                            Align::Min,
                            &title,
                            1.0,
                            if st.target { col } else { pal().ink },
                        );
                        let root = st.chord.as_deref().and_then(chord_root);
                        let shape: [Option<u8>; 6] =
                            std::array::from_fn(|s| st.notes.iter().find(|n| n.0 == s as i32).map(|n| n.1.clamp(0, 24) as u8));
                        let moved = st.moved.clone();
                        let gb = Rect::from_min_max(Pos2::new(r.min.x, r.min.y + 18.0 * sc), r.max);
                        crate::fretboard::chord_box_coloured(p, gb, &shape, &|m| {
                            let txt =
                                root.map_or(String::new(), |rt| crate::fretboard::degree(m.rem_euclid(12), rt).0.to_string());
                            (txt, if moved.contains(&m) { col } else { Color32::from_gray(245) })
                        });
                    }
                });
                // the neck: the grip, what is in reach around it that fits the chord, the move's notes
                let mut dots = focus_dots(&grip, chord.as_deref(), &beats, picked_mark.map(|m| m.0));
                // the grip clicked above: its notes ringed brightly, the rest dimmed
                if let Some(st) = self.cm_step.filter(|s| s.0 == from).and_then(|s| steps.get(s.1)) {
                    for d in dots.iter_mut() {
                        d.strength *= 0.45;
                    }
                    for n in &st.notes {
                        let spot = (n.0 as usize, n.1.clamp(0, 24) as u8);
                        match dots.iter_mut().find(|d| d.spot == spot) {
                            Some(d) => {
                                d.strength = 1.0;
                                d.ring = false;
                                d.col = col;
                            }
                            None => dots.push(crate::improv::NeckDot {
                                spot,
                                text: String::new(),
                                col,
                                strength: 1.0,
                                ring: false,
                                root: false,
                            }),
                        }
                    }
                }
                crate::improv::neck_view(ui, &dots, None);
                // what it is: the move, and each chord's notes
                if let Some(m) = picked_mark {
                    crate::views::dim_line(ui, EXPLAIN[m.0.min(9)], 1.0, pal().ink2);
                }
                let mut said = 0;
                let mut last: Option<String> = None;
                for bt in &beats {
                    let Some(c) = bt.chord.clone().or_else(|| if last.is_none() { chord.clone() } else { None }) else {
                        continue;
                    };
                    if last.as_ref() == Some(&c) || said >= 4 {
                        continue;
                    }
                    let r = chord_root(&c);
                    let mut ns: Vec<i32> = bt.notes.iter().map(|n| n[0]).collect();
                    ns.sort();
                    ns.dedup();
                    let names: Vec<String> = ns
                        .iter()
                        .map(|m| match r {
                            Some(r) => {
                                format!("{} {}", theory::note_name(*m, sharps), crate::fretboard::degree(m.rem_euclid(12), r).0)
                            }
                            None => theory::note_name(*m, sharps).to_string(),
                        })
                        .collect();
                    crate::views::dim_line(
                        ui,
                        &format!(
                            "{}:  {}   (TOP: {})",
                            crate::chart::pretty_chord(&c),
                            names.join("  "),
                            names.last().cloned().unwrap_or_default()
                        ),
                        1.0,
                        pal().ink,
                    );
                    last = Some(c);
                    said += 1;
                }
                // a smooth connection: each voice's step into the next chord
                if picked_mark.is_some_and(|m| m.0 == 7) {
                    let first = beats.iter().find(|b| b.notes.len() >= 3);
                    let lastb = beats.iter().rev().find(|b| b.notes.len() >= 3);
                    if let (Some(x), Some(y)) = (first, lastb) {
                        let mut p: Vec<i32> = x.notes.iter().map(|n| n[0]).collect();
                        let mut q: Vec<i32> = y.notes.iter().map(|n| n[0]).collect();
                        p.sort();
                        q.sort();
                        let steps: Vec<String> = p
                            .iter()
                            .zip(&q)
                            .map(|(a, b)| {
                                let d = b - a;
                                format!(
                                    "{}>{} ({})",
                                    theory::note_name(*a, sharps),
                                    theory::note_name(*b, sharps),
                                    if d == 0 { "stays".to_string() } else { format!("{:+}", d) }
                                )
                            })
                            .collect();
                        crate::views::dim_line(ui, &format!("THE VOICES, LOW TO HIGH:  {}", steps.join("   ")), 1.0, pal().ink);
                    }
                }
            });
        });
        if close {
            self.cm_sel = None;
            self.cm_sel2 = None;
            self.cm_mark = None;
        }
    }

    /// Start playing the bars picked, from the first.
    fn cm_start(&mut self, t: &Tune) {
        let spb = 60.0 / self.cm_bpm.max(20) as f32;
        let mut ev: Vec<(usize, i32, f32, f32)> = Vec::new();
        let mut q0 = 0.0f32;
        let mut start_q = 0.0f32;
        let mut end_q = 0.0f32;
        for (bi, b) in t.bars.iter().enumerate() {
            let bar_q = b.time.0 as f32 * 4.0 / b.time.1.max(1) as f32;
            if bi == self.cm_from {
                start_q = q0;
            }
            if bi >= self.cm_from && bi <= self.cm_to {
                for k in &b.beats {
                    for n in &k.notes {
                        if n[3] == 0 {
                            ev.push(((n[1].clamp(0, 5)) as usize, n[0], (q0 + k.at - start_q) * spb, 0.85));
                        }
                    }
                }
                end_q = q0 + bar_q;
            }
            q0 += bar_q;
        }
        if ev.is_empty() {
            return;
        }
        let len = (end_q - start_q) * spb;
        self.player.send(Cmd::StopOnce);
        self.player.send(Cmd::Once(crate::band::string_events(&ev, len), 0.9));
        self.cm_play = Some((Instant::now(), start_q, end_q));
    }

    /// The beat sounding now, if playing (going round again when looping).
    fn cm_now(&mut self, t: &Tune) -> Option<Spot> {
        let (t0, start_q, end_q) = self.cm_play?;
        let spb = 60.0 / self.cm_bpm.max(20) as f32;
        let q = start_q + t0.elapsed().as_secs_f32() / spb;
        if q >= end_q {
            if self.cm_loop {
                self.cm_start(t);
            } else {
                self.cm_play = None;
            }
            return None;
        }
        self.ctx.request_repaint();
        let mut q0 = 0.0f32;
        for (bi, b) in t.bars.iter().enumerate() {
            let bar_q = b.time.0 as f32 * 4.0 / b.time.1.max(1) as f32;
            if q < q0 + bar_q {
                let k = b.beats.iter().rposition(|k| q0 + k.at <= q + 1e-3)?;
                return Some((bi, k));
            }
            q0 += bar_q;
        }
        None
    }

    /// Bars laid out in rows - chord names, grips, the notes written out (stems, beams, rests, dots, ties), tab, note
    /// names, each as shown - every beat given the room it needs, sized with the zoom. `live`: the song's own bars
    /// (the loop shown, moves marked, beats picked); says which beat and which move the pointer is on.
    fn draw_bars(
        &self,
        ui: &mut egui::Ui,
        bars: &[transcription::Bar],
        grips: &[transcription::Grip],
        sharps: bool,
        now: Option<Spot>,
        live: bool,
        marks: &[(usize, Spot, Spot)],
        first_bar: usize,
    ) -> BarsOut {
        let show = self.cm_show;
        let sc = crate::font::ui_scale();
        let g = 7.0 * sc; // the staff's line gap
        let hw = 0.62 * g; // half a note head
        let tg = 10.0 * sc; // the tab's line gap
        let grip_w = 44.0 * sc;
        let mut out = BarsOut::default();
        // the moves' labels, in lanes so overlapping ones sit one above the other (four lanes at most)
        let mut lane_end: Vec<Spot> = Vec::new();
        let lanes: Vec<Option<usize>> = marks
            .iter()
            .map(|m| match lane_end.iter().position(|e| *e < m.1) {
                Some(l) => {
                    lane_end[l] = m.2;
                    Some(l)
                }
                None if lane_end.len() < 4 => {
                    lane_end.push(m.2);
                    Some(lane_end.len() - 1)
                }
                None => None,
            })
            .collect();
        let lane_h = 12.0 * sc;
        let marks_h = if lane_end.is_empty() { 0.0 } else { lane_end.len() as f32 * lane_h + 3.0 * sc };
        let (names_h, grip_h, tab_h, nn_h) = (
            if show[0] { 20.0 * sc } else { 0.0 },
            if show[1] { 62.0 * sc } else { 0.0 },
            if show[3] { 6.0 * tg + 8.0 * sc } else { 0.0 },
            if show[4] { 16.0 * sc } else { 0.0 },
        );
        let w = ui.available_width().min(ui.clip_rect().max.x - ui.cursor().min.x - 4.0).max(300.0);
        let name_w = |c: &str| crate::font::text_w(&crate::chart::pretty_chord(c), 1.5) + 10.0 * sc;
        let num_w = crate::font::text_w("888", 1.0) + 4.0 * sc;
        // the bars as written: notes and rests, stems up or down, beamed in beats
        let items: Vec<Vec<Item>> = bars.iter().map(|b| bar_items(b, sharps, g, show[3])).collect();
        // each placed after the one before with room for its length (a long note more), its accidentals, seconds
        // and dots - and a chord name (or its grip) never under the one before it
        let dur_w = |len: f32| g * (1.5 + 2.2 * len.max(0.125).sqrt());
        let chord_of = |b: &transcription::Bar, it: &Item| {
            it.beat.filter(|_| it.first).and_then(|k| b.beats.get(k)).and_then(|k| k.chord.clone())
        };
        // a time signature where the bars start and wherever it changes
        let ts_at = |b: &transcription::Bar| {
            let i = bars.iter().position(|x| std::ptr::eq(x, b)).unwrap_or(0);
            i == 0 || bars[i - 1].time != b.time
        };
        // a key signature again where it changes (the start of each line has its own)
        let kw_at = |b: &transcription::Bar| {
            let i = bars.iter().position(|x| std::ptr::eq(x, b)).unwrap_or(0);
            if show[2] && i > 0 && bars[i - 1].key != b.key {
                key_w(b.key.map_or(0, |k| k as i32), g)
            } else {
                0.0
            }
        };
        let layout: Vec<(Vec<f32>, f32)> = bars
            .iter()
            .zip(&items)
            .map(|(b, its)| {
                let mut xs = Vec::with_capacity(its.len());
                let lead = num_w + kw_at(b) + if show[2] && ts_at(b) { 2.4 * g } else { 0.0 };
                let mut prev: Option<(f32, &Item)> = None;
                let mut last_chord: Option<(f32, f32)> = None;
                for it in its {
                    let mut x = match prev {
                        None => lead + it.left.max(g),
                        Some((px, p)) => px + dur_w(p.len).max(p.right + it.left + 0.6 * g),
                    };
                    let chord = chord_of(b, it);
                    if let (Some(_), Some((cx, cw))) = (&chord, last_chord) {
                        x = x.max(cx + cw);
                    }
                    if let Some(c) = &chord {
                        let cw = if show[0] { name_w(c) } else { 0.0 }.max(if show[1] { grip_w + 6.0 * sc } else { 0.0 });
                        last_chord = Some((x, cw));
                    }
                    xs.push(x);
                    prev = Some((x, it));
                }
                let end = prev.map_or(4.0 * g, |(px, p)| px + dur_w(p.len).max(p.right + g));
                (xs, end.max(last_chord.map_or(0.0, |c| c.0 + c.1)))
            })
            .collect();
        let clip = ui.clip_rect();
        let pointer = ui.input(|i| i.pointer.hover_pos()).filter(|pp| clip.contains(*pp));
        let whole = live && self.cm_from == 0 && self.cm_to + 1 >= bars.len();
        // what is picked: the beats being dragged over, or the ones clicked
        let picked =
            self.cm_drag
                .or(self.cm_sel.map(|a| (a, self.cm_sel2.unwrap_or(a))))
                .map(|(a, z)| if a <= z { (a, z) } else { (z, a) });
        let key_room = bars.iter().filter_map(|b| b.key).map(|k| key_w(k as i32, g)).fold(0.0, f32::max);
        let left = if show[2] { 8.0 + 4.4 * g + key_room } else { 10.0 * sc };
        let ink = pal().ink;
        let line = Stroke::new(1.0_f32, pal().ink2);
        let ys_of = |bot: f32, step: i32| bot - (step - 30) as f32 * g / 2.0;
        let mut bi = 0;
        while bi < bars.len() {
            // a row: as many bars as fit
            let mut row: Vec<usize> = Vec::new();
            let mut used = left;
            while bi < bars.len() {
                let bw = layout[bi].1;
                if !row.is_empty() && used + bw > w {
                    break;
                }
                row.push(bi);
                used += bw;
                bi += 1;
            }
            // the room the notes need above and below the staff
            let (mut hi, mut lo) = (40, 28);
            for &b in &row {
                for it in &items[b] {
                    if let (Some(top), Some(bot)) = (it.heads.last(), it.heads.first()) {
                        let stem = if it.base < 4.0 { 7 } else { 0 };
                        hi = hi.max(top.step + if it.up { stem } else { 0 } + 1);
                        lo = lo.min(bot.step - if it.up { 0 } else { stem } - 1);
                    }
                }
            }
            let above = if show[2] { (hi - 38) as f32 * g / 2.0 + 6.0 * sc } else { 0.0 };
            let staff_h = if show[2] { above + 4.0 * g + (30 - lo) as f32 * g / 2.0 + 6.0 * sc } else { 0.0 };
            let row_h = marks_h + names_h + grip_h + staff_h + tab_h + nn_h + 10.0 * sc;
            let (rect, _) = ui.allocate_exact_size(Vec2::new(w, row_h), Sense::click_and_drag());
            if !ui.is_rect_visible(rect) {
                continue;
            }
            let p = ui.painter();
            p.rect_filled(rect.shrink2(Vec2::new(0.0, 2.0)), 3.0, pal().beige_lt);
            let top = rect.min.y + 2.0;
            let body_top = top + marks_h;
            let names_y = body_top + 10.0 * sc;
            let grip_top = body_top + names_h + 2.0 * sc;
            let staff_top = body_top + names_h + grip_h + above;
            let staff_bot = staff_top + 4.0 * g;
            let tab_top = body_top + names_h + grip_h + staff_h + 6.0 * sc;
            let tab_y = |s: usize| tab_top + (5 - s) as f32 * tg;
            let nn_y = body_top + names_h + grip_h + staff_h + tab_h + 8.0 * sc;
            let ys = |step: i32| ys_of(staff_bot, step);
            let x_end = rect.min.x + used;
            if show[2] {
                for k in 0..5 {
                    let y = staff_top + k as f32 * g;
                    p.line_segment([Pos2::new(rect.min.x + 4.0, y), Pos2::new(x_end, y)], line);
                }
                crate::pixmap(
                    p,
                    Pos2::new(rect.min.x + 8.0, staff_top - 1.7 * g),
                    7.4 * g / crate::lines_ui::CLEF.len() as f32,
                    &crate::lines_ui::CLEF,
                    ink,
                );
                // the key at the start of the line
                if let Some(k) = bars[row[0]].key {
                    key_sig(p, rect.min.x + 8.0 + 3.9 * g, staff_bot, k as i32, g, ink);
                }
            }
            if show[3] {
                for s in 0..6 {
                    p.line_segment([Pos2::new(rect.min.x + 4.0, tab_y(s)), Pos2::new(x_end, tab_y(s))], line);
                }
            }
            // the last head of each pitch, for ties
            let mut heads_at: Vec<(i32, Pos2)> = Vec::new();
            let mut x = rect.min.x + left;
            for &b in &row {
                let bar = &bars[b];
                let its = &items[b];
                let xs = &layout[b].0;
                let bw = layout[b].1;
                let in_loop = live && b >= self.cm_from && b <= self.cm_to;
                // the bars looped (when not the whole song)
                if in_loop && !whole {
                    p.rect_filled(
                        Rect::from_min_max(Pos2::new(x, body_top), Pos2::new(x + bw, rect.max.y - 2.0)),
                        0.0,
                        pal().red.gamma_multiply(0.07),
                    );
                }
                ptext(
                    p,
                    Pos2::new(x + 2.0, names_y),
                    Align::Min,
                    &(first_bar + b + 1).to_string(),
                    1.0,
                    if in_loop && !whole { pal().red } else { pal().dim },
                );
                let beat_x = |k: usize| its.iter().position(|it| it.beat == Some(k)).map_or(x, |i| x + xs[i]);
                let kw = if b > 0 && bars[b - 1].key != bar.key { key_w(bar.key.map_or(0, |k| k as i32), g) } else { 0.0 };
                if show[2] && kw > 0.0 && b != row[0] {
                    key_sig(p, x + num_w - 0.4 * g, staff_bot, bar.key.map_or(0, |k| k as i32), g, ink);
                }
                if show[2] && (b == 0 || bars[b - 1].time != bar.time) {
                    let tx = x + num_w + kw + 1.0 * g;
                    ptext(p, Pos2::new(tx, staff_top + g), Align::Center, &bar.time.0.to_string(), 2.0, ink);
                    ptext(p, Pos2::new(tx, staff_top + 3.0 * g), Align::Center, &bar.time.1.to_string(), 2.0, ink);
                }
                // the moves spotted here: a soft band over their beats, their name above
                for (mi, m) in marks.iter().enumerate() {
                    if b < m.1 .0 || b > m.2 .0 {
                        continue;
                    }
                    let x0 = if b == m.1 .0 { beat_x(m.1 .1) - 1.6 * hw } else { x };
                    let x1 = if b == m.2 .0 {
                        let i = its.iter().rposition(|it| it.beat == Some(m.2 .1)).unwrap_or(0);
                        xs.get(i + 1).map_or(x + bw, |n| x + n - 1.6 * hw)
                    } else {
                        x + bw
                    };
                    let (cr, cg, cb) = MARK_COLS[m.0.min(9)];
                    let col = Color32::from_rgb(cr, cg, cb);
                    let on = self.cm_mark == Some(mi);
                    let band = Rect::from_min_max(Pos2::new(x0, body_top), Pos2::new(x1.max(x0 + 6.0), rect.max.y - 2.0));
                    p.rect_filled(band, 2.0, col.gamma_multiply(if on { 0.22 } else { 0.1 }));
                    if on {
                        p.rect_stroke(band, 2.0, Stroke::new(1.5_f32, col));
                    }
                    if let Some(l) = lanes[mi] {
                        let chip = Rect::from_min_max(
                            Pos2::new(x0, top + 2.0 + l as f32 * lane_h),
                            Pos2::new(x1.max(x0 + 6.0), top + (l + 1) as f32 * lane_h),
                        );
                        p.rect_filled(chip, 2.0, col.gamma_multiply(if on { 0.9 } else { 0.55 }));
                        let name = SHORT[m.0.min(9)];
                        if (b == m.1 .0 || b == row[0]) && crate::font::text_w(name, 1.0) + 6.0 < chip.width() {
                            ptext(
                                p,
                                Pos2::new(chip.min.x + 3.0, chip.center().y),
                                Align::Min,
                                name,
                                1.0,
                                Color32::from_gray(250),
                            );
                        }
                        if live && pointer.is_some_and(|pp| chip.contains(pp)) {
                            out.chip = Some(mi);
                        }
                    }
                    if live && !on && pointer.is_some_and(|pp| band.contains(pp)) {
                        out.band = Some(mi);
                    }
                }
                // what a group's beam needs: (stem x, far head y, near head y, up, flags, triplet)
                let mut beams: Vec<(usize, f32, f32, f32, bool, u8, bool)> = Vec::new();
                for (i, it) in its.iter().enumerate() {
                    let bx = x + xs[i];
                    // in the move picked: its notes in its colour
                    let ink = match (self.cm_mark.and_then(|mi| marks.get(mi)), it.beat) {
                        (Some(m), Some(k)) if (b, k) >= m.1 && (b, k) <= m.2 => {
                            let (r, g2, bl) = MARK_COLS[m.0.min(9)];
                            Color32::from_rgb(r, g2, bl)
                        }
                        _ => ink,
                    };
                    // a beat's column: lit when sounding, picked, or under the pointer
                    if let Some(k) = it.beat {
                        let here = (b, k);
                        let nx = xs.get(i + 1).map_or(x + bw, |n| x + n);
                        let col_rect = Rect::from_min_max(
                            Pos2::new(bx - 1.6 * hw, body_top),
                            Pos2::new((nx - 1.6 * hw).max(bx + hw), rect.max.y - 2.0),
                        );
                        if it.first {
                            if now == Some(here) {
                                p.rect_filled(col_rect, 2.0, pal().red.gamma_multiply(0.2));
                            } else if live && picked.is_some_and(|(lo, hi)| here >= lo && here <= hi) {
                                p.rect_filled(col_rect, 2.0, pal().sel.gamma_multiply(0.6));
                            }
                        }
                        if live && pointer.is_some_and(|pp| col_rect.contains(pp)) {
                            out.hover = Some(here);
                        }
                    }
                    // the chord name, and its grip (the one the transcription draws, else the notes played)
                    if let Some(c) = chord_of(bar, it) {
                        if show[0] {
                            ptext(
                                p,
                                Pos2::new(bx - 4.0 * sc, names_y + if live && i == 0 { 0.0 } else { 0.0 }),
                                Align::Min,
                                &crate::chart::pretty_chord(&c),
                                1.5,
                                ink,
                            );
                        }
                        if show[1] {
                            let shape: [Option<u8>; 6] = match grips.iter().find(|g| g.name == c) {
                                Some(gr) => gr.frets.map(|f| if f < 0 { None } else { Some(f as u8) }),
                                None => {
                                    let mut s = [None; 6];
                                    for n in it.beat.and_then(|k| bar.beats.get(k)).map(|k| k.notes.as_slice()).unwrap_or(&[]) {
                                        if (0..6).contains(&n[1]) {
                                            s[n[1] as usize] = Some(n[2].clamp(0, 24) as u8);
                                        }
                                    }
                                    s
                                }
                            };
                            if shape.iter().any(|f| f.is_some()) {
                                let gb =
                                    Rect::from_min_size(Pos2::new(bx - 4.0 * sc, grip_top), Vec2::new(grip_w, grip_h - 6.0 * sc));
                                if live && pointer.is_some_and(|pp| gb.contains(pp)) {
                                    if let Some(k) = it.beat {
                                        out.grip = Some((b, k));
                                        p.rect_filled(gb.expand(2.0), 2.0, pal().sel.gamma_multiply(0.5));
                                    }
                                }
                                crate::fretboard::chord_box_labelled(p, gb, &shape, &|_| String::new());
                            }
                        }
                    }
                    if show[2] {
                        if it.heads.is_empty() {
                            rest(p, Pos2::new(bx, staff_top + 2.0 * g), it.base, g, ink);
                        } else {
                            let dx = |h: &Head| bx + h.side as f32 * 2.0 * hw * 0.95;
                            let left_shift = if it.heads.iter().any(|h| h.side < 0) { 2.0 * hw } else { 0.0 };
                            let right_shift = if it.heads.iter().any(|h| h.side > 0) { 2.0 * hw } else { 0.0 };
                            for h in &it.heads {
                                let hx = dx(h);
                                let y = ys(h.step);
                                // ledger lines
                                let mut s = 28;
                                while s >= h.step {
                                    p.line_segment([Pos2::new(hx - 1.7 * hw, ys(s)), Pos2::new(hx + 1.7 * hw, ys(s))], line);
                                    s -= 2;
                                }
                                let mut s = 40;
                                while s <= h.step {
                                    p.line_segment([Pos2::new(hx - 1.7 * hw, ys(s)), Pos2::new(hx + 1.7 * hw, ys(s))], line);
                                    s += 2;
                                }
                                if it.base >= 2.0 {
                                    p.circle_stroke(Pos2::new(hx, y), hw * 0.82, Stroke::new(1.5_f32, ink));
                                } else {
                                    p.circle_filled(Pos2::new(hx, y), hw * 0.9, ink);
                                }
                                if let Some(a) = h.mark {
                                    let ax = bx - hw - left_shift - 0.6 * g - (h.acc_col.max(1) - 1) as f32 * 1.1 * g;
                                    crate::lines_ui::accidental(p, Pos2::new(ax, y), a, g, ink);
                                }
                                // dots (off a line, into the space above)
                                let dy = if h.step % 2 == 0 { -g / 2.0 } else { 0.0 };
                                for d in 0..it.dots {
                                    p.circle_filled(
                                        Pos2::new(bx + hw + right_shift + 0.5 * g + d as f32 * 0.55 * g, y + dy),
                                        0.17 * g,
                                        ink,
                                    );
                                }
                                // a tie from the same note before
                                if h.tied {
                                    let from = heads_at
                                        .iter()
                                        .find(|a| a.0 == h.midi)
                                        .map(|a| a.1)
                                        .unwrap_or(Pos2::new(rect.min.x + left - g, y));
                                    let s = if it.up { 1.0 } else { -1.0 };
                                    let a = Pos2::new(from.x + hw, from.y + s * 0.6 * g);
                                    let z = Pos2::new(hx - hw, y + s * 0.6 * g);
                                    let mid = Pos2::new((a.x + z.x) / 2.0, (a.y + z.y) / 2.0 + s * 0.9 * g);
                                    p.add(egui::epaint::QuadraticBezierShape::from_points_stroke(
                                        [a, mid, z],
                                        false,
                                        Color32::TRANSPARENT,
                                        Stroke::new(1.2_f32, ink),
                                    ));
                                }
                            }
                            for h in &it.heads {
                                let at = Pos2::new(dx(h), ys(h.step));
                                match heads_at.iter_mut().find(|a| a.0 == h.midi) {
                                    Some(a) => a.1 = at,
                                    None => heads_at.push((h.midi, at)),
                                }
                            }
                            // the stem, and the flags of a note on its own
                            if it.base < 4.0 {
                                let (lo_y, hi_y) = (ys(it.heads[0].step), ys(it.heads[it.heads.len() - 1].step));
                                let (near, far) = if it.up { (lo_y, hi_y) } else { (hi_y, lo_y) };
                                let sx = if it.up { bx + hw * 0.85 } else { bx - hw * 0.85 };
                                match it.group {
                                    Some(gid) => beams.push((gid, sx, far, near, it.up, it.flags, it.trip)),
                                    None => {
                                        let s = if it.up { -1.0 } else { 1.0 };
                                        let end = far + s * 3.5 * g;
                                        p.line_segment([Pos2::new(sx, near), Pos2::new(sx, end)], Stroke::new(1.2_f32, ink));
                                        for f in 0..it.flags {
                                            let y0 = end - s * f as f32 * 0.75 * g;
                                            p.add(egui::Shape::line(
                                                vec![
                                                    Pos2::new(sx, y0),
                                                    Pos2::new(sx + 0.9 * g, y0 - s * 1.0 * g),
                                                    Pos2::new(sx + 0.7 * g, y0 - s * 2.0 * g),
                                                ],
                                                Stroke::new(1.4_f32, ink),
                                            ));
                                        }
                                        if it.trip {
                                            ptext(p, Pos2::new(sx, end + s * 0.8 * g), Align::Center, "3", 1.0, pal().ink2);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // the tab (a tied note in brackets) and the top note's name
                    if let Some(k) = it.beat.filter(|_| it.first) {
                        let notes = &bar.beats[k].notes;
                        if show[3] {
                            for n in notes.iter().filter(|n| (0..6).contains(&n[1])) {
                                let txt = if n[3] == 1 { format!("({})", n[2]) } else { n[2].to_string() };
                                let c = Pos2::new(bx, tab_y(n[1] as usize));
                                p.rect_filled(
                                    Rect::from_center_size(c, Vec2::new(crate::font::text_w(&txt, 1.0) + 3.0, tg * 0.9)),
                                    0.0,
                                    pal().beige_lt,
                                );
                                ptext(p, c, Align::Center, &txt, 1.0, if n[3] == 1 { pal().dim } else { ink });
                            }
                        }
                        if show[4] {
                            if let Some(n) = notes.iter().max_by_key(|n| n[0]) {
                                ptext(p, Pos2::new(bx, nn_y), Align::Center, theory::note_name(n[0], sharps), 1.0, pal().ink2);
                            }
                        }
                    }
                }
                // the beams: one line over a group's stems (a little slant at most), the shorter notes' beams below it
                let mut gi = 0;
                while gi < beams.len() {
                    let gid = beams[gi].0;
                    let n = beams[gi..].iter().take_while(|m| m.0 == gid).count();
                    let ms = &beams[gi..gi + n];
                    gi += n;
                    let up = ms[0].4;
                    let s = if up { -1.0 } else { 1.0 };
                    let (x0, xn) = (ms[0].1, ms[n - 1].1);
                    let mut ya = ms[0].2 + s * 3.5 * g;
                    let yb0 = ms[n - 1].2 + s * 3.5 * g;
                    let mut yb = ya + (yb0 - ya).clamp(-g, g);
                    let at = |ya: f32, yb: f32, x: f32| ya + (yb - ya) * (x - x0) / (xn - x0).max(1.0);
                    // every stem at least this long
                    let mut push: f32 = 0.0;
                    for m in ms {
                        let need = m.2 + s * 2.8 * g;
                        let lv = at(ya, yb, m.1);
                        push = push.max(if up { lv - need } else { need - lv });
                    }
                    ya += s * push;
                    yb += s * push;
                    let bt = 0.45 * g;
                    for m in ms {
                        p.line_segment([Pos2::new(m.1, m.3), Pos2::new(m.1, at(ya, yb, m.1))], Stroke::new(1.2_f32, ink));
                    }
                    let beam = |xa: f32, xz: f32, lvl: u8| {
                        let off = -s * (lvl - 1) as f32 * 0.75 * g;
                        let (ya2, yz2) = (at(ya, yb, xa) + off, at(ya, yb, xz) + off);
                        p.add(egui::Shape::convex_polygon(
                            vec![
                                Pos2::new(xa, ya2),
                                Pos2::new(xz, yz2),
                                Pos2::new(xz, yz2 - s * bt),
                                Pos2::new(xa, ya2 - s * bt),
                            ],
                            ink,
                            Stroke::NONE,
                        ));
                    };
                    beam(x0, xn, 1);
                    for lvl in 2..=3u8 {
                        for i in 0..n {
                            if ms[i].5 < lvl {
                                continue;
                            }
                            if i + 1 < n && ms[i + 1].5 >= lvl {
                                beam(ms[i].1, ms[i + 1].1, lvl);
                            } else if !(i > 0 && ms[i - 1].5 >= lvl) {
                                let stub = 0.9 * g;
                                if i + 1 < n {
                                    beam(ms[i].1, ms[i].1 + stub, lvl);
                                } else {
                                    beam(ms[i].1 - stub, ms[i].1, lvl);
                                }
                            }
                        }
                    }
                    if ms.iter().any(|m| m.6) {
                        let mx = (x0 + xn) / 2.0;
                        ptext(p, Pos2::new(mx, at(ya, yb, mx) + s * 0.9 * g), Align::Center, "3", 1.0, pal().ink2);
                    }
                }
                x += bw;
                // the bar line (a double one where the song ends)
                if show[2] {
                    p.line_segment([Pos2::new(x, staff_top), Pos2::new(x, staff_bot)], line);
                    if live && b + 1 == bars.len() {
                        p.line_segment([Pos2::new(x - 0.5 * g, staff_top), Pos2::new(x - 0.5 * g, staff_bot)], line);
                        p.line_segment([Pos2::new(x, staff_top), Pos2::new(x, staff_bot)], Stroke::new(0.5 * g, ink));
                    }
                }
                if show[3] {
                    p.line_segment([Pos2::new(x, tab_y(5)), Pos2::new(x, tab_y(0))], line);
                }
            }
        }
        out
    }

    // ------------------------------------------------------------------ MOVES

    fn cm_moves_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        para(ui, "The moves you kept from your transcriptions. Click one to see it, play it, hear it in another key, star the ones to work on. In SONGS, the coloured strips over the bars are moves Backline spots - click one to keep it.", pal().ink2);
        let _ = acts;
        // the moves kept
        ui.add_space(8.0);
        section_header(ui, &format!("YOUR MOVES  ({})", self.cm_moves.len()));
        ui.horizontal_wrapped(|ui| {
            let mut kinds: Vec<&str> = vec!["ALL KINDS"];
            kinds.extend(KINDS.iter());
            if let Some(i) = crate::dropdown_fixed(ui, "cm_kind", &kinds, self.cm_kind as usize, 300.0) {
                self.cm_kind = i as u8;
            }
            if crate::check_box(ui, "STARRED ONLY", self.cm_star_only).clicked() {
                self.cm_star_only = !self.cm_star_only;
            }
            crate::views::label(ui, "KEY", 40.0);
            const SHIFTS: [&str; 13] = ["-6", "-5", "-4", "-3", "-2", "-1", "AS PLAYED", "+1", "+2", "+3", "+4", "+5", "+6"];
            if let Some(i) = crate::dropdown_fixed(ui, "cm_shift", &SHIFTS, (self.cm_shift + 6).clamp(0, 12) as usize, 130.0) {
                self.cm_shift = i as i32 - 6;
            }
        });
        let mut drop: Option<usize> = None;
        let mut star: Option<usize> = None;
        let mut open: Option<usize> = None;
        let shift = self.cm_shift;
        for i in 0..self.cm_moves.len() {
            let m = self.cm_moves[i].clone();
            if self.cm_star_only && !m.star {
                continue;
            }
            if self.cm_kind > 0 && m.kind != self.cm_kind as usize - 1 {
                continue;
            }
            ui.horizontal(|ui| {
                if retro_btn_w(ui, if m.star { "*" } else { " " }, 26.0, m.star).tip("Star it").clicked() {
                    star = Some(i);
                }
                if retro_btn_w(ui, "PLAY", 60.0, false).clicked() {
                    self.cm_play_beats(&m.beats, shift);
                }
                let chords: Vec<String> = m.beats.iter().filter_map(|b| shifted(b, shift).chord).collect();
                let label = format!(
                    "{}   ({}, BARS {}-{})   {}",
                    m.name.to_uppercase(),
                    KINDS.get(m.kind).copied().unwrap_or("MY OWN"),
                    m.bars.0,
                    m.bars.1,
                    chords.join(" ")
                );
                let open_now = self.cm_open == Some(i);
                let state = if open_now { RowState::Playing } else { RowState::Normal };
                if let Some(r) = list_row(ui, i, None, || label.clone(), String::new, state, false, false) {
                    if r.clicked() {
                        open = Some(i);
                    }
                    r.context_menu(|ui| {
                        if menu_item(ui, "Forget this move") {
                            drop = Some(i);
                            ui.close_menu();
                        }
                    });
                }
            });
        }
        if let Some(i) = open {
            self.cm_open = if self.cm_open == Some(i) { None } else { Some(i) };
        }
        if let Some(i) = star {
            self.cm_moves[i].star = !self.cm_moves[i].star;
            transcription::save_moves(&self.cm_moves);
        }
        // the move open: drawn like the songs, in the key picked
        if let Some(m) = self.cm_open.and_then(|i| self.cm_moves.get(i)).cloned() {
            ui.add_space(6.0);
            section_header(ui, &m.name.to_uppercase());
            let beats: Vec<Beat> = m.beats.iter().map(|b| shifted(b, shift)).collect();
            let grips: Vec<transcription::Grip> = m
                .grips
                .iter()
                .map(|g| {
                    let mut g = g.clone();
                    g.name = crate::chart::transpose(&g.name, shift);
                    let lo = g.frets.iter().filter(|f| **f > 0).map(|f| *f as i32 + shift).min().unwrap_or(1);
                    let adj = if lo < 1 { 12 } else { 0 };
                    g.frets = g.frets.map(|f| {
                        if f < 0 {
                            f
                        } else if f == 0 && shift == 0 {
                            0
                        } else {
                            (f as i32 + shift + adj).max(0) as i8
                        }
                    });
                    g
                })
                .collect();
            let sharps = beats.iter().find_map(|b| b.chord.as_deref().and_then(chord_root)).map_or(false, lines::key_sharps);
            let bars = as_bars(&beats);
            let _ = self.draw_bars(ui, &bars, &grips, sharps, None, false, &[], 0);
        }
        if let Some(i) = drop {
            self.cm_moves.remove(i);
            transcription::save_moves(&self.cm_moves);
        }
    }

    /// Play some beats, `shift` semitones away.
    fn cm_play_beats(&mut self, beats: &[Beat], shift: i32) {
        let spb = 60.0 / self.cm_bpm.max(20) as f32;
        let mut ev: Vec<(usize, i32, f32, f32)> = Vec::new();
        let mut end = 0.0f32;
        for b in beats {
            let b = shifted(b, shift);
            for n in &b.notes {
                if n[3] == 0 {
                    ev.push((n[1].clamp(0, 5) as usize, n[0], b.at * spb, 0.85));
                }
            }
            end = end.max((b.at + b.len) * spb);
        }
        if !ev.is_empty() {
            self.player.send(Cmd::StopOnce);
            self.player.send(Cmd::Once(crate::band::string_events(&ev, end + 0.8), 0.9));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcription::Bar;

    fn chords(names: &[&str]) -> Tune {
        Tune {
            bars: vec![Bar {
                key: None,
                time: (4, 4),
                beats: names
                    .iter()
                    .enumerate()
                    .map(|(i, c)| Beat { at: i as f32, len: 1.0, notes: vec![[60, 3, 5, 0]], chord: Some(c.to_string()) })
                    .collect(),
            }],
            ..Default::default()
        }
    }

    /// How many of each kind of move the transcriptions in the share folder hold (run by hand).
    #[test]
    #[ignore]
    fn counts_the_moves() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("share").join("transcriptions");
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let Ok(mut t) = transcription::load(&e.path()) else { continue };
            name_from_notes(&mut t);
            let mut n = [0; 10];
            for m in suggest(&t) {
                n[m.0] += 1;
            }
            eprintln!("{:50} {:?}", t.title, n);
        }
    }

    #[test]
    fn spots_the_usual_moves() {
        let ii_v_i = suggest(&chords(&["Dm7", "G7", "Cmaj7"]));
        assert!(ii_v_i.iter().any(|m| m.0 == 0));
        let dim = suggest(&chords(&["Cmaj7", "C#dim7", "Dm7"]));
        assert!(dim.iter().any(|m| m.0 == 1));
        let sub = suggest(&chords(&["Dm7", "Db7", "Cmaj7"]));
        assert!(sub.iter().any(|m| m.0 == 2));
    }

    #[test]
    fn moves_transpose_on_their_strings() {
        let b = Beat { at: 0.0, len: 1.0, notes: vec![[52, 0, 0, 0]], chord: Some("Em".into()) };
        let up = shifted(&b, 2);
        assert_eq!(up.notes[0], [54, 0, 2, 0]);
        assert_eq!(up.chord.as_deref(), Some("F#m"));
    }
}
