//! The pure parts of SUPERIMPOSE, shared by the desktop and the phone: the shapes that can go over a chord, the
//! scales a chord is heard in, the shapes that fit one (best first), and the usual choices with why.

/// The shapes that can go over a chord: name, notes from its own root.
pub(crate) const SI_SHAPES: [(&str, &[i32]); 13] = [
    ("MINOR PENTATONIC", &[0, 3, 5, 7, 10]),
    ("MAJOR PENTATONIC", &[0, 2, 4, 7, 9]),
    ("MAJOR TRIAD", &[0, 4, 7]),
    ("MINOR TRIAD", &[0, 3, 7]),
    ("AUGMENTED TRIAD", &[0, 4, 8]),
    ("DIMINISHED TRIAD", &[0, 3, 6]),
    ("MAJ7", &[0, 4, 7, 11]),
    ("7", &[0, 4, 7, 10]),
    ("M7", &[0, 3, 7, 10]),
    ("M(MAJ7)", &[0, 3, 7, 11]),
    ("M7\u{266D}5", &[0, 3, 6, 10]),
    ("DIM7", &[0, 3, 6, 9]),
    ("MINOR 6TH PENTATONIC", &[0, 3, 5, 7, 9]),
];

/// The scales a chord can be heard in, the usual first: name, steps.
pub(crate) const SI_SCALES: [(&str, [i32; 8], usize); 20] = [
    ("IONIAN (MAJOR)", [0, 2, 4, 5, 7, 9, 11, 0], 7),
    ("LYDIAN (#11)", [0, 2, 4, 6, 7, 9, 11, 0], 7),
    ("MIXOLYDIAN", [0, 2, 4, 5, 7, 9, 10, 0], 7),
    ("LYDIAN DOMINANT (#11)", [0, 2, 4, 6, 7, 9, 10, 0], 7),
    ("ALTERED (\u{266D}9 #9 #11 \u{266D}13)", [0, 1, 3, 4, 6, 8, 10, 0], 7),
    ("HALF-WHOLE DIMINISHED (\u{266D}9 #9 #11 13)", [0, 1, 3, 4, 6, 7, 9, 10], 8),
    ("WHOLE TONE (9 #11 #5)", [0, 2, 4, 6, 8, 10, 0, 0], 6),
    ("PHRYGIAN DOMINANT (\u{266D}9 \u{266D}13)", [0, 1, 4, 5, 7, 8, 10, 0], 7),
    ("MIXOLYDIAN \u{266D}6 (\u{266D}13)", [0, 2, 4, 5, 7, 8, 10, 0], 7),
    ("DORIAN", [0, 2, 3, 5, 7, 9, 10, 0], 7),
    ("AEOLIAN (NATURAL MINOR)", [0, 2, 3, 5, 7, 8, 10, 0], 7),
    ("PHRYGIAN (\u{266D}9)", [0, 1, 3, 5, 7, 8, 10, 0], 7),
    ("LOCRIAN NATURAL 9", [0, 2, 3, 5, 6, 8, 10, 0], 7),
    ("LOCRIAN", [0, 1, 3, 5, 6, 8, 10, 0], 7),
    ("MELODIC MINOR", [0, 2, 3, 5, 7, 9, 11, 0], 7),
    ("HARMONIC MINOR", [0, 2, 3, 5, 7, 8, 11, 0], 7),
    ("WHOLE-HALF DIMINISHED", [0, 2, 3, 5, 6, 8, 9, 11], 8),
    ("LYDIAN AUGMENTED (#11 #5)", [0, 2, 4, 6, 8, 9, 11, 0], 7),
    ("HARMONIC MAJOR (\u{266D}13)", [0, 2, 4, 5, 7, 8, 11, 0], 7),
    ("AUGMENTED (SYMMETRIC)", [0, 3, 4, 7, 8, 11, 0, 0], 6),
];

/// The scales that fit a chord (its notes from the root): every note of the chord in the scale - except a dominant's
/// 5th, which the altered, whole-tone and diminished sounds change. Indexes into SI_SCALES, the usual first.
pub(crate) fn si_scales_for(chord: &[i32]) -> Vec<usize> {
    let dominant = chord.contains(&4) && chord.contains(&10);
    let mut fits: Vec<usize> = (0..SI_SCALES.len())
        .filter(|i| {
            let (_, steps, n) = SI_SCALES[*i];
            let s = &steps[..n];
            chord.iter().all(|c| s.contains(&c.rem_euclid(12)) || (dominant && c.rem_euclid(12) == 7))
        })
        .collect();
    // the standard choice first: whole-half for a diminished 7th, whole tone for an augmented triad or a 7#5,
    // Lydian augmented for a maj7#5, half-whole for a 7b9 (with no b13)
    let has = |x: i32| chord.contains(&x);
    let first: Option<&str> = if has(3) && has(6) && has(9) && !has(4) {
        Some("WHOLE-HALF")
    } else if has(4) && has(8) && has(11) {
        Some("LYDIAN AUGMENTED")
    } else if has(4) && has(8) && !has(7) {
        Some("WHOLE TONE")
    } else if dominant && has(1) && !has(8) {
        Some("HALF-WHOLE")
    } else {
        None
    };
    // and a scale with both 3rds (b3 and 3) is never the usual one over a minor chord (nor one with a b3 over a
    // major chord that isn't a dominant): those go after the rest
    let minor = has(3) && !has(4);
    let major = has(4) && !has(3) && !has(10);
    let clash = |i: usize| {
        let s = &SI_SCALES[i].1[..SI_SCALES[i].2];
        (minor && s.contains(&4)) || (major && s.contains(&3))
    };
    fits.sort_by_key(|i| (first.map_or(true, |f| !SI_SCALES[*i].0.starts_with(f)), clash(*i)));
    fits
}

/// Every shape on every note that fits inside the scale, best first: the ones that add the chord's colours
/// (tensions) while still touching its 3rd or 7th lead; one that only restates the chord comes last. At most 14.
/// (shape, its root from the chord's root).
pub(crate) fn si_auto(chord: &[i32], scale: &[i32]) -> Vec<(usize, i32)> {
    let mut found: Vec<(i32, usize, i32)> = Vec::new();
    for (k, (_, notes)) in SI_SHAPES.iter().enumerate() {
        for off in 0..12 {
            let pcs: Vec<i32> = notes.iter().map(|x| (x + off).rem_euclid(12)).collect();
            if !pcs.iter().all(|p| scale.contains(p)) {
                continue;
            }
            let tensions = pcs.iter().filter(|p| !chord.contains(p)).count() as i32;
            let guide = pcs.iter().any(|p| chord.iter().any(|c| (*c == 3 || *c == 4 || *c == 10 || *c == 11) && c == p)) as i32;
            let score = tensions * 2 + guide * 3 + if notes.len() == 5 { 1 } else { 0 } - if tensions == 0 { 6 } else { 0 };
            found.push((score, k, off));
        }
    }
    found.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    found.into_iter().take(14).map(|(_, k, o)| (k, o)).collect()
}

/// The usual choices over each chord: (shape, its root from the chord's root, why).
pub(crate) fn si_suggestions(quality: usize) -> Vec<(usize, i32, &'static str)> {
    match quality {
        // maj7
        0 => vec![
            (1, 0, "THE MAJOR PENTATONIC ON THE ROOT: SAFE, NO 4TH TO CLASH"),
            (1, 7, "MAJOR PENTATONIC FROM THE 5TH: ADDS THE 7TH AND THE 9TH"),
            (1, 2, "MAJOR PENTATONIC A STEP UP: THE LYDIAN SOUND (#11)"),
            (0, 4, "MINOR PENTATONIC FROM THE 3RD: THE SAME NOTES AS THE 5TH'S MAJOR, IN MINOR SHAPES"),
            (0, 11, "MINOR PENTATONIC FROM THE 7TH: LYDIAN AGAIN (#11), BRIGHT"),
            (8, 4, "THE M7 FROM THE 3RD: 3 5 7 9, THE CHORD WITHOUT ITS ROOT PLUS THE 9TH"),
            (2, 2, "A MAJOR TRIAD A STEP UP: 9 #11 13, THE LYDIAN UPPER STRUCTURE"),
        ],
        // 7
        1 => vec![
            (0, 7, "MINOR PENTATONIC FROM THE 5TH: 5 \u{266D}7 R 9 11, A SUS FLAVOUR"),
            (1, 0, "MAJOR PENTATONIC ON THE ROOT: R 9 3 5 13, SWEET"),
            (0, 9, "MINOR PENTATONIC FROM THE 6TH: 13 R 9 3 5, THE SAME NOTES LAID OUT AGAIN"),
            (9, 7, "M(MAJ7) FROM THE 5TH: 5 \u{266D}7 9 #11, FROM ITS MELODIC MINOR (LYDIAN DOMINANT)"),
            (2, 2, "MAJOR TRIAD A STEP UP: 9 #11 13, THE LYDIAN DOMINANT UPPER STRUCTURE"),
            (1, 6, "MAJOR PENTATONIC A TRITONE AWAY: ALL THE ALTERATIONS (#11 \u{266D}13 \u{266D}7 \u{266D}9 #9)"),
            (2, 9, "MAJOR TRIAD ON THE 6TH: 13 \u{266D}9 3, THE DIMINISHED SOUND"),
        ],
        // 7alt
        2 => vec![
            (1, 6, "MAJOR PENTATONIC A TRITONE AWAY: #11 \u{266D}13 \u{266D}7 \u{266D}9 #9, THE ALTERED SOUND"),
            (0, 3, "MINOR PENTATONIC FROM THE \u{266D}3: THE SAME NOTES, IN MINOR SHAPES"),
            (2, 8, "MAJOR TRIAD ON THE \u{266D}6: \u{266D}13 R #9"),
            (2, 6, "MAJOR TRIAD ON THE \u{266D}5: #11 \u{266D}7 \u{266D}9"),
            (9, 1, "M(MAJ7) A HALF STEP UP: THE ALTERED SCALE IS ITS MELODIC MINOR"),
            (4, 0, "AUGMENTED TRIAD ON THE ROOT: R 3 #5, THE WHOLE-TONE COLOUR"),
        ],
        // 7#11
        3 => vec![
            (9, 7, "M(MAJ7) FROM THE 5TH: 5 \u{266D}7 9 #11"),
            (2, 2, "MAJOR TRIAD A STEP UP: 9 #11 13"),
            (1, 0, "MAJOR PENTATONIC ON THE ROOT: R 9 3 5 13. ADD THE #11 AS A PASSING NOTE."),
            (0, 7, "MINOR PENTATONIC FROM THE 5TH: 5 \u{266D}7 R 9 11. THE 11 RUBS, SO LAND ON #11 INSTEAD."),
        ],
        // m7
        4 => vec![
            (0, 0, "THE MINOR PENTATONIC ON THE ROOT: HOME BASE"),
            (0, 2, "MINOR PENTATONIC A STEP UP: 9 11 5 13 R, THE DORIAN SOUND"),
            (0, 7, "MINOR PENTATONIC FROM THE 5TH: 5 \u{266D}7 R 9 11, OPEN AND MODERN"),
            (6, 3, "MAJ7 FROM THE \u{266D}3: \u{266D}3 5 \u{266D}7 9, THE CHORD WITH ITS 9TH"),
            (2, 10, "MAJOR TRIAD ON THE \u{266D}7: \u{266D}7 9 11"),
            (2, 5, "MAJOR TRIAD ON THE 4TH: 11 13 R, DORIAN"),
        ],
        // m7b5
        5 => vec![
            (12, 3, "MINOR 6TH PENTATONIC FROM THE \u{266D}3: \u{266D}3 \u{266D}5 \u{266D}6 \u{266D}7 R"),
            (2, 10, "MAJOR TRIAD ON THE \u{266D}7: \u{266D}7 9 11, LOCRIAN #2"),
            (9, 3, "M(MAJ7) FROM THE \u{266D}3: \u{266D}3 \u{266D}5 \u{266D}7 9"),
        ],
        // m(maj7)
        6 => vec![
            (2, 7, "MAJOR TRIAD FROM THE 5TH: 5 7 9"),
            (3, 2, "MINOR TRIAD A STEP UP: 9 11 13, MELODIC MINOR"),
            (4, 3, "AUGMENTED TRIAD FROM THE \u{266D}3: \u{266D}3 5 7"),
            (12, 0, "MINOR 6TH PENTATONIC ON THE ROOT: R \u{266D}3 4 5 6, MELODIC MINOR"),
        ],
        // maj7#11
        _ => vec![
            (1, 2, "MAJOR PENTATONIC A STEP UP: 9 3 #11 13 7"),
            (2, 2, "MAJOR TRIAD A STEP UP: 9 #11 13"),
            (0, 11, "MINOR PENTATONIC FROM THE 7TH: 7 9 3 #11 13"),
            (1, 7, "MAJOR PENTATONIC FROM THE 5TH: 5 13 7 9 3"),
        ],
    }
}
