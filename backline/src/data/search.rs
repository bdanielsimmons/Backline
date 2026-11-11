//! Matching what you type against names: folded text (no case, accents or punctuation), small typos forgiven,
//! the closest first (`filter_score`, `filtered`), and a tune's proper title from a short name (`likely_title`).

/// Text for matching: lower case, accents off (o for ō), punctuation dropped (Monk's is monks).
pub(crate) fn fold(s: &str) -> String {
    s.chars()
        .flat_map(|c| c.to_lowercase())
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' => 'a',
            'ç' => 'c',
            'è' | 'é' | 'ê' | 'ë' | 'ē' => 'e',
            'ì' | 'í' | 'î' | 'ï' | 'ī' => 'i',
            'ñ' => 'n',
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' => 'o',
            'ù' | 'ú' | 'û' | 'ü' | 'ū' => 'u',
            'ý' | 'ÿ' => 'y',
            c => c,
        })
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect()
}

/// Letters to change, add or drop to turn `a` into `b` (stops counting past `max`).
pub(crate) fn edits(a: &[char], b: &[char], max: usize) -> usize {
    if a.len().abs_diff(b.len()) > max {
        return max + 1;
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut row = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            row[j] = (prev[j] + 1).min(row[j - 1] + 1).min(prev[j - 1] + cost);
        }
        if row.iter().min().copied().unwrap_or(0) > max {
            return max + 1;
        }
        prev = row;
    }
    prev[b.len()]
}

/// Does every word of the filter match one of these? Forgiving: accents and punctuation aside, any order, a word
/// may be the start of one (while typing), or one off a word by a typo or two (thelonius finds Thelonious).
#[cfg(test)]
pub(crate) fn matches_filter(filter: &str, fields: &[&str]) -> bool {
    filter_score(filter, fields).is_some()
}

/// How well these match the filter (None: not at all). Lower is better: 0 for every word found as typed, more for
/// each typo - so the closest come first.
pub(crate) fn filter_score(filter: &str, fields: &[&str]) -> Option<u32> {
    let hay = fold(&fields.join(" "));
    let words: Vec<Vec<char>> = hay.split_whitespace().map(|w| w.chars().collect()).collect();
    let mut score = 0;
    for q in fold(filter).split_whitespace() {
        let qc: Vec<char> = q.chars().collect();
        score += if hay.contains(q) {
            0
        } else {
            // short words must be right; longer ones may be a letter (or two) off
            let max = match qc.len() {
                0..=3 => return None,
                4..=6 => 1,
                _ => 2,
            };
            let best = words
                .iter()
                .map(|w| {
                    let whole = edits(&qc, w, max);
                    // or the start of a longer word, typo and all
                    let start = if w.len() > qc.len() { edits(&qc, &w[..qc.len()], max) } else { max + 1 };
                    whole.min(start)
                })
                .min()
                .unwrap_or(max + 1);
            if best > max {
                return None;
            }
            3 + best as u32
        };
    }
    Some(score)
}

/// A tune's proper title, when the name typed is short of it or misspelled ("Incompatibilidade de" for
/// "Incompatibilidade de Gênios"): the title most of these (the recordings LOOK UP found) go by, among those that
/// match what was typed, with the tunes Backline knows (`known`) counted once each. None when nothing matches, two
/// titles are as common, or the best is the name as typed (capitals aside).
pub(crate) fn likely_title(name: &str, titles: &[&str], known: &[&str]) -> Option<String> {
    let squash = |s: &str| fold(s).split_whitespace().collect::<Vec<_>>().join(" ");
    let typed = squash(name);
    if typed.is_empty() {
        return None;
    }
    let fits = |c: &str| {
        let f = squash(c);
        !f.is_empty() && (f.starts_with(&typed) || filter_score(name, &[c]).is_some())
    };
    // (the title folded, how many in all, each spelling and how many)
    let mut groups: Vec<(String, u32, Vec<(String, u32)>)> = Vec::new();
    let mut add = |c: &str| {
        let f = squash(c);
        let gi = groups.iter().position(|g| g.0 == f).unwrap_or_else(|| {
            groups.push((f, 0, Vec::new()));
            groups.len() - 1
        });
        let g = &mut groups[gi];
        g.1 += 1;
        match g.2.iter_mut().find(|s| s.0 == c) {
            Some(s) => s.1 += 1,
            None => g.2.push((c.to_string(), 1)),
        }
    };
    for t in titles {
        // "Autumn Leaves (Live)", or "Artist - Title": the part that matches
        if let Some(c) = t.split(" - ").map(crate::chart::clean_title).find(|c| fits(c)) {
            add(&c);
        }
    }
    for k in known.iter().filter(|k| fits(k)) {
        add(k);
    }
    groups.sort_by_key(|a| std::cmp::Reverse(a.1));
    if groups.len() > 1 && groups[0].1 == groups[1].1 {
        return None;
    }
    let g = groups.into_iter().next()?;
    // the commonest spelling (the first found, of those as common)
    let mut best = g.2[0].clone();
    for s in &g.2[1..] {
        if s.1 > best.1 {
            best = s.clone();
        }
    }
    (best.0.to_lowercase() != name.trim().to_lowercase()).then_some(best.0)
}

thread_local! {
    /// The last filtered list: (filter, how many there were, which list, the order of the ones that match), so a
    /// long library is matched once per change, not every frame.
    static FILTERED: std::cell::RefCell<(String, usize, u64, Vec<usize>)> = const { std::cell::RefCell::new((String::new(), 0, 0, Vec::new())) };
}

/// The items that match the filter, the closest first (worked out once per filter and list).
pub(crate) fn filtered<T>(filter: &str, items: &[T], fields: impl Fn(&T) -> Vec<String>, key: u64) -> Vec<usize> {
    FILTERED.with(|c| {
        let mut c = c.borrow_mut();
        if c.0 != filter || c.1 != items.len() || c.2 != key {
            let mut hits: Vec<(u32, usize)> = items
                .iter()
                .enumerate()
                .filter_map(|(i, it)| {
                    let f = fields(it);
                    let refs: Vec<&str> = f.iter().map(String::as_str).collect();
                    filter_score(filter, &refs).map(|s| (s, i))
                })
                .collect();
            hits.sort_by_key(|h| h.0);
            *c = (filter.to_string(), items.len(), key, hits.into_iter().map(|h| h.1).collect());
        }
        c.3.clone()
    })
}
