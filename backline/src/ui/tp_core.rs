//! The pure parts of TRIAD PAIRS: the pairs of triads that fit a chord (best first, from every scale the chord is
//! heard in), the triads' voicings (by pitch, or on three neighbouring strings), the exercises played with a pair,
//! and one line through a run of changes, a pair for each chord.
//!
//! A triad pair is two triads with six different notes between them: played one after the other they make a
//! six-note scale whose sound is the two triads, not runs of steps. Pitches are MIDI numbers; pitch classes and
//! roots are 0-11, a chord's notes counted from its own root.

use crate::si_core::{si_scales_for, SI_SCALES};

/// A place on the neck: (string, 0 = low E; fret).
pub(crate) type Spot = (usize, u8);

/// The kinds of triad: name, what goes after the root in a chord symbol, the notes from its root.
pub(crate) const KINDS: [(&str, &str, [i32; 3]); 4] =
    [("MAJOR", "", [0, 4, 7]), ("MINOR", "m", [0, 3, 7]), ("AUGMENTED", "+", [0, 4, 8]), ("DIMINISHED", "dim", [0, 3, 6])];

/// A triad: its root (0-11) and kind (an index into `KINDS`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Triad {
    pub root: i32,
    pub kind: u8,
}

impl Triad {
    pub(crate) fn new(root: i32, kind: u8) -> Triad {
        Triad { root: root.rem_euclid(12), kind: kind.min(3) }
    }

    /// Its three pitch classes, root first.
    pub(crate) fn pcs(&self) -> [i32; 3] {
        KINDS[self.kind as usize].2.map(|x| (self.root + x).rem_euclid(12))
    }

    /// The same triad moved up `by` half steps.
    pub(crate) fn moved(&self, by: i32) -> Triad {
        Triad::new(self.root + by, self.kind)
    }
}

/// A pair that fits a chord: the two triads (the one rooted nearer above the chord's root first) and the scale
/// they both come from (an index into `SI_SCALES`). Roots are counted from the chord's root.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Fit {
    pub a: Triad,
    pub b: Triad,
    pub scale: usize,
}

/// Whether a note (from the chord's root) is one to avoid landing on over the chord: a half step above one of its
/// notes - except a dominant's b9, #9 and b13, which are its colours.
pub(crate) fn avoid(chord: &[i32], r: i32) -> bool {
    let r = r.rem_euclid(12);
    let has = |x: i32| chord.iter().any(|c| c.rem_euclid(12) == x);
    let dominant = has(4) && has(10);
    !has(r) && has((r - 1).rem_euclid(12)) && !(dominant && matches!(r, 1 | 3 | 8))
}

/// The pairs that fit a chord (its notes from its root), best first, at most 12: two triads with six different
/// notes, both inside a scale the chord is heard in. Major and minor triads (augmented ones too, where a scale has
/// them); diminished ones only when nothing else gives a pair. What counts: the colours the pair adds over the
/// chord, touching its 3rd or 7th so the line stays tied to it, no note to avoid, the two triads a step apart (the
/// classic pair), and the more usual scale.
pub(crate) fn pairs_for(chord: &[i32]) -> Vec<Fit> {
    let chord: Vec<i32> = chord.iter().map(|c| c.rem_euclid(12)).collect();
    let scales = si_scales_for(&chord);
    for with_dim in [false, true] {
        // (score, notes to avoid in it, the pair)
        let mut found: Vec<(i32, i32, Fit)> = Vec::new();
        for (rank, si) in scales.iter().enumerate() {
            let (_, steps, n) = SI_SCALES[*si];
            let scale = &steps[..n];
            // the scale's triads
            let mut triads: Vec<Triad> = Vec::new();
            for root in 0..12 {
                for kind in 0..if with_dim { 4 } else { 3 } {
                    let t = Triad::new(root, kind);
                    if t.pcs().iter().all(|p| scale.contains(p)) {
                        triads.push(t);
                    }
                }
            }
            for (i, x) in triads.iter().enumerate() {
                for y in &triads[i + 1..] {
                    if x.pcs().iter().any(|p| y.pcs().contains(p)) {
                        continue;
                    }
                    let all: Vec<i32> = x.pcs().iter().chain(y.pcs().iter()).copied().collect();
                    // (three colours is the most that counts: more and the pair is outside the chord, not over it)
                    let colour = (all.iter().filter(|p| !chord.contains(p)).count() as i32).min(3);
                    let anchor = all.iter().any(|p| chord.contains(p) && matches!(p, 3 | 4 | 10 | 11)) as i32;
                    let avoids = all.iter().filter(|p| avoid(&chord, **p)).count() as i32;
                    let gap = (x.root - y.root).rem_euclid(12).min((y.root - x.root).rem_euclid(12));
                    let step = (gap == 1 || gap == 2) as i32;
                    let odd = (x.kind == 2) as i32 + (y.kind == 2) as i32 + 2 * ((x.kind == 3) as i32 + (y.kind == 3) as i32);
                    let score = colour * 2 + anchor * 3 - avoids * 5 + step * 2 - odd * 2 - rank.min(6) as i32;
                    // the one rooted nearer above the chord's root first
                    let (a, b) = if x.root <= y.root { (*x, *y) } else { (*y, *x) };
                    let fit = Fit { a, b, scale: *si };
                    match found.iter_mut().find(|f| f.2.a == a && f.2.b == b) {
                        // the same pair from another scale: kept with the one it scores best in
                        Some(f) => {
                            if score > f.0 {
                                *f = (score, avoids, fit);
                            }
                        }
                        None => found.push((score, avoids, fit)),
                    }
                }
            }
        }
        // a pair with a note to avoid only when there is nothing else
        if found.iter().any(|f| f.1 == 0) {
            found.retain(|f| f.1 == 0);
        }
        if !found.is_empty() {
            // best first; level ones in the more usual scale, then from the lower root
            let rank_of = |s: usize| scales.iter().position(|x| *x == s).unwrap_or(99);
            found.sort_by(|p, q| q.0.cmp(&p.0).then(rank_of(p.2.scale).cmp(&rank_of(q.2.scale))).then(p.2.a.root.cmp(&q.2.a.root)));
            return found.into_iter().take(12).map(|f| f.2).collect();
        }
    }
    Vec::new()
}

/// The pair to play over a chord when nothing is picked: the best one, or failing any, its own triad and the one a
/// step above it.
pub(crate) fn best_pair(chord: &[i32]) -> (Triad, Triad) {
    if let Some(f) = pairs_for(chord).first() {
        return (f.a, f.b);
    }
    let minor = chord.contains(&3) && !chord.contains(&4);
    let t = Triad::new(0, minor as u8);
    (t, t.moved(2))
}

/// One voicing of a triad: its three pitches, low to high, and where they are played when it sits on a string set.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Voicing {
    pub notes: [i32; 3],
    pub spots: Option<[Spot; 3]>,
    /// which of the pair: false the first, true the second
    pub second: bool,
}

/// A triad's close voicings (the three notes within an octave, any inversion) with the lowest note from `lo` to
/// `hi`, lowest first.
pub(crate) fn by_pitch(t: Triad, lo: i32, hi: i32, second: bool) -> Vec<Voicing> {
    let pcs = t.pcs();
    (lo..=hi)
        .filter(|m| pcs.contains(&m.rem_euclid(12)))
        .map(|m| {
            // the next two notes of the triad above it
            let mut v = [m, 0, 0];
            let mut x = m;
            for k in 1..3 {
                x += 1;
                while !pcs.contains(&x.rem_euclid(12)) {
                    x += 1;
                }
                v[k] = x;
            }
            Voicing { notes: v, spots: None, second }
        })
        .collect()
}

/// A triad's close voicings on three neighbouring strings (`low`: the lowest of them, 0 = low E), one note a string,
/// no fret past `top` and no wider than four frets, lowest first.
pub(crate) fn on_strings(t: Triad, low: usize, tuning: &[i32; 6], top: u8, second: bool) -> Vec<Voicing> {
    let low = low.min(3);
    let mut out: Vec<Voicing> = Vec::new();
    for v in by_pitch(t, tuning[low], tuning[low] + top as i32, second) {
        let frets: Vec<i32> = (0..3).map(|k| v.notes[k] - tuning[low + k]).collect();
        let (mn, mx) = (*frets.iter().min().unwrap(), *frets.iter().max().unwrap());
        if mn < 0 || mx > top as i32 || mx - mn > 4 {
            continue;
        }
        let spots = [0, 1, 2].map(|k| (low + k, frets[k] as u8));
        out.push(Voicing { spots: Some(spots), ..v });
    }
    out
}

/// The pair's voicings taken in turn, the first triad then the second, each one starting above the last, from the
/// lowest up: at most `max` of them.
pub(crate) fn alternate(va: &[Voicing], vb: &[Voicing], max: usize) -> Vec<Voicing> {
    let mut out: Vec<Voicing> = Vec::new();
    let mut last = i32::MIN;
    let mut turn_b = false;
    while out.len() < max {
        let list = if turn_b { vb } else { va };
        match list.iter().find(|v| v.notes[0] > last) {
            Some(v) => {
                last = v.notes[0];
                out.push(*v);
                turn_b = !turn_b;
            }
            None => break,
        }
    }
    out
}

/// The exercises: (id, name, what it does, whether it works on a string set).
pub(crate) const EXERCISES: [(u8, &str, &str, bool); 6] = [
    (0, "ALTERNATE", "One triad, then the other, up the inversions and back.", true),
    (1, "ZIGZAG", "The first triad up, the second down, all the way.", true),
    (2, "DOWN FIRST", "As ALTERNATE, from the top: down first, then back up.", true),
    (3, "NOTE BY NOTE", "One note of each triad in turn: seconds and thirds.", true),
    (4, "SPREAD", "Each triad open: root, 5th, the 3rd an octave up.", false),
    (5, "AS A SCALE", "All six notes as a scale, two octaves up and back.", false),
];

/// An exercise with a pair, as pitches and (on a string set) where each is played. `va`/`vb`: the two triads'
/// voicings, lowest first; `pcs`: the six notes, for AS A SCALE; `lo`: where the scale starts.
pub(crate) fn exercise(ex: u8, va: &[Voicing], vb: &[Voicing], pcs: &[i32], lo: i32) -> Vec<(i32, Option<Spot>)> {
    let run = alternate(va, vb, 8);
    let up = |v: &Voicing| -> Vec<(i32, Option<Spot>)> { (0..3).map(|k| (v.notes[k], v.spots.map(|s| s[k]))).collect() };
    let down = |v: &Voicing| -> Vec<(i32, Option<Spot>)> { up(v).into_iter().rev().collect() };
    let mut out: Vec<(i32, Option<Spot>)> = Vec::new();
    match ex {
        1 => {
            for (i, v) in run.iter().enumerate() {
                out.extend(if i % 2 == 0 { up(v) } else { down(v) });
            }
            for (i, v) in run.iter().enumerate().rev().skip(1) {
                out.extend(if i % 2 == 0 { down(v) } else { up(v) });
            }
        }
        2 => {
            for v in run.iter().rev() {
                out.extend(down(v));
            }
            for v in run.iter().skip(1) {
                out.extend(up(v));
            }
        }
        3 => {
            // a voicing of the first and the one of the second above it, note by note
            let pairs: Vec<(&Voicing, &Voicing)> = run.chunks(2).filter(|c| c.len() == 2).map(|c| (&c[0], &c[1])).collect();
            for (a, b) in &pairs {
                for k in 0..3 {
                    out.push((a.notes[k], a.spots.map(|s| s[k])));
                    out.push((b.notes[k], b.spots.map(|s| s[k])));
                }
            }
            for (a, b) in pairs.iter().rev() {
                for k in (0..3).rev() {
                    out.push((b.notes[k], b.spots.map(|s| s[k])));
                    out.push((a.notes[k], a.spots.map(|s| s[k])));
                }
            }
        }
        4 => {
            // close voicing a b c: a, c, then b an octave up
            let spread = |v: &Voicing| [v.notes[0], v.notes[2], v.notes[1] + 12];
            let run: Vec<&Voicing> = run.iter().take(6).collect();
            for v in &run {
                out.extend(spread(v).iter().map(|m| (*m, None)));
            }
            for v in run.iter().rev() {
                out.extend(spread(v).iter().rev().map(|m| (*m, None)));
            }
        }
        5 => {
            let up: Vec<i32> = (lo..lo + 25).filter(|m| pcs.contains(&m.rem_euclid(12))).take(13).collect();
            out.extend(up.iter().map(|m| (*m, None)));
            out.extend(up.iter().rev().skip(1).map(|m| (*m, None)));
        }
        _ => {
            for v in &run {
                out.extend(up(v));
            }
            for v in run.iter().rev().skip(1) {
                out.extend(down(v));
            }
        }
    }
    out
}

/// One line through changes: for each chord its two triads' voicings and how many notes it lasts. The line goes
/// on from where it is - the voicing of the chord's first triad nearest the last note, then the second, and so on
/// in turn, each a step on in the way it is going - turning round at either end of the voicings there are.
pub(crate) fn through(chords: &[(Vec<Voicing>, Vec<Voicing>, usize)]) -> Vec<(i32, Option<Spot>)> {
    let mut out: Vec<(i32, Option<Spot>)> = Vec::new();
    let mut up_dir = true;
    for (va, vb, len) in chords {
        let start = out.len();
        if va.is_empty() || vb.is_empty() {
            // (nothing to play it with: the last note held over, so the chords after stay in their bars)
            let last = out.last().copied().unwrap_or((60, None));
            out.extend(std::iter::repeat(last).take(*len));
            continue;
        }
        // where to start: the first triad's voicing whose first note (going the way the line goes) is nearest the
        // last note; the lowest to begin with
        let mut cur: Voicing = match out.last() {
            Some((m, _)) => *va.iter().min_by_key(|v| ((if up_dir { v.notes[0] } else { v.notes[2] }) - m).abs()).unwrap(),
            None => va[0],
        };
        while out.len() - start < *len {
            let notes: Vec<(i32, Option<Spot>)> = (0..3).map(|k| (cur.notes[k], cur.spots.map(|s| s[k]))).collect();
            if up_dir {
                out.extend(notes);
            } else {
                out.extend(notes.into_iter().rev());
            }
            // the other triad's next voicing on in this direction; none left: turn round
            let other = if cur.second { va } else { vb };
            let next = |up: bool| -> Option<Voicing> {
                if up {
                    other.iter().find(|v| v.notes[0] > cur.notes[0]).copied()
                } else {
                    other.iter().rev().find(|v| v.notes[0] < cur.notes[0]).copied()
                }
            };
            cur = match next(up_dir) {
                Some(v) => v,
                None => {
                    up_dir = !up_dir;
                    match next(up_dir) {
                        Some(v) => v,
                        None => break,
                    }
                }
            };
        }
        out.truncate(start + *len);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const STD: [i32; 6] = [40, 45, 50, 55, 59, 64];

    fn names(f: &Fit) -> String {
        let n = |t: Triad| format!("{}{}", ["C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B"][t.root as usize], KINDS[t.kind as usize].1);
        format!("{} + {}", n(f.a), n(f.b))
    }

    #[test]
    fn the_usual_pairs_come_first() {
        // Cmaj7: the Lydian pairs C + D and D + Em (9 #11 13 over it), nothing with the F that rubs on the 3rd
        let maj7 = pairs_for(&[0, 4, 7, 11]);
        let top: Vec<String> = maj7.iter().take(2).map(names).collect();
        assert_eq!(top, ["C + D", "D + Em"], "{:?}", maj7.iter().map(names).collect::<Vec<_>>());
        assert!(maj7.iter().all(|f| !f.a.pcs().contains(&5) && !f.b.pcs().contains(&5)), "{:?}", maj7.iter().map(names).collect::<Vec<_>>());
        // C7: no pair landing on the 4th, and the first one colours it (the Lydian dominant's C + D), not the
        // altered sound, which is a choice to make, not the usual one
        let dom = pairs_for(&[0, 4, 7, 10]);
        assert_eq!(names(&dom[0]), "C + D", "{:?}", dom.iter().map(names).collect::<Vec<_>>());
        assert!(dom.iter().take(3).all(|f| !f.a.pcs().contains(&5) && !f.b.pcs().contains(&5)));
        // C7alt (altered): F# + Ab, the pair from its melodic minor
        let alt = pairs_for(&[0, 4, 10, 1, 3, 8]);
        assert!(alt.iter().take(3).any(|f| names(f) == "Gb + Ab"), "{:?}", alt.iter().map(names).collect::<Vec<_>>());
        // every pair: six different notes
        for chord in [vec![0, 3, 7, 10], vec![0, 3, 6, 10], vec![0, 3, 6, 9], vec![0, 4, 8], vec![0, 3, 7, 11]] {
            let ps = pairs_for(&chord);
            assert!(!ps.is_empty(), "{:?}", chord);
            for f in ps {
                let mut all: Vec<i32> = f.a.pcs().iter().chain(f.b.pcs().iter()).copied().collect();
                all.sort();
                all.dedup();
                assert_eq!(all.len(), 6, "{}", names(&f));
            }
        }
    }

    #[test]
    fn voicings_on_a_string_set() {
        // C major on the top three strings: every one is three of C E G, one a string, a playable span
        let v = on_strings(Triad::new(0, 0), 3, &STD, 15, false);
        assert!(v.len() >= 4);
        for x in &v {
            let s = x.spots.unwrap();
            assert_eq!([s[0].0, s[1].0, s[2].0], [3, 4, 5]);
            for k in 0..3 {
                assert_eq!(STD[s[k].0] + s[k].1 as i32, x.notes[k]);
                assert!([0, 4, 7].contains(&x.notes[k].rem_euclid(12)));
            }
        }
        assert!(v.windows(2).all(|w| w[0].notes[0] < w[1].notes[0]));
    }

    #[test]
    fn alternate_takes_turns_upward() {
        let a = by_pitch(Triad::new(0, 0), 48, 72, false);
        let b = by_pitch(Triad::new(2, 0), 48, 72, true);
        let run = alternate(&a, &b, 8);
        assert_eq!(run.len(), 8);
        for (i, v) in run.iter().enumerate() {
            assert_eq!(v.second, i % 2 == 1);
        }
        assert!(run.windows(2).all(|w| w[0].notes[0] < w[1].notes[0]));
        // every exercise gives a line, there and back
        for (ex, ..) in EXERCISES {
            let line = exercise(ex, &a, &b, &[0, 2, 4, 6, 7, 9], 52);
            assert!(line.len() >= 12, "{ex}");
            assert_eq!(line.first().map(|n| n.0), line.last().map(|n| n.0), "{ex}");
        }
    }

    #[test]
    fn a_line_through_the_changes_lasts_each_chord() {
        // ii-V-I in C, eighth notes: 8 notes a bar
        let mk = |rel: &[i32], root: i32| {
            let (a, b) = best_pair(rel);
            (by_pitch(a.moved(root), 52, 69, false), by_pitch(b.moved(root), 52, 69, true), 8)
        };
        let chords = vec![mk(&[0, 3, 7, 10], 2), mk(&[0, 4, 7, 10], 7), mk(&[0, 4, 7, 11], 0), mk(&[0, 4, 7, 11], 0)];
        let line = through(&chords);
        assert_eq!(line.len(), 32);
        // each bar's notes are its own pair's
        for (bar, (va, vb, _)) in chords.iter().enumerate() {
            let pcs: Vec<i32> = va[0].notes.iter().chain(vb[0].notes.iter()).map(|m| m.rem_euclid(12)).collect();
            for n in &line[bar * 8..bar * 8 + 8] {
                assert!(pcs.contains(&n.0.rem_euclid(12)), "bar {bar}");
            }
        }
        // no leap wider than an octave between bars
        assert!(line.windows(2).all(|w| (w[0].0 - w[1].0).abs() <= 12));
    }
}
