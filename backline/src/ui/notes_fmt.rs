//! Formatting in notes (the diary's, a tune's): the marks are kept in the text itself. `# ` starts a heading line,
//! `- ` a list line, and `---` is a line between sections. Inline there are `**bold**`, `__underline__` and
//! `[name](https://...)` for a link; a bare web address is a link too. Pure: what the marks mean, and the tools that
//! put them in or take them out.

/// What a line is.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Block {
    Para,
    Heading,
    Item,
    Rule,
}

/// A line's kind, and where its words start (after `# ` or `- `), in chars.
pub(crate) fn block_of(line: &[char]) -> (Block, usize) {
    if line.starts_with(&['#', ' ']) {
        (Block::Heading, 2)
    } else if line.starts_with(&['-', ' ']) {
        (Block::Item, 2)
    } else if line.iter().collect::<String>().trim_end() == "---" {
        (Block::Rule, line.len())
    } else {
        (Block::Para, 0)
    }
}

/// How one character is shown.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub(crate) struct Look {
    /// a mark (`**`, `# `...): faint while writing, gone when reading
    pub mark: bool,
    pub bold: bool,
    pub under: bool,
    pub link: bool,
    /// in a heading line
    pub head: bool,
}

/// A run of a line's characters, `s..e` in chars, shown one way; `url` for a link.
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Span {
    pub s: usize,
    pub e: usize,
    pub look: Look,
    pub url: Option<String>,
}

fn starts(line: &[char], at: usize, pat: &str) -> bool {
    let p: Vec<char> = pat.chars().collect();
    line.len() >= at + p.len() && line[at..at + p.len()] == p[..]
}

/// Where a web address starting at `at` ends: at a space, less any punctuation closing a sentence (and a `)` without
/// its `(`).
fn url_end(line: &[char], at: usize) -> usize {
    let mut e = at;
    while e < line.len() && !line[e].is_whitespace() {
        e += 1;
    }
    loop {
        let Some(&c) = line[at..e].last() else { break };
        let open = line[at..e].iter().filter(|c| **c == '(').count();
        let close = line[at..e].iter().filter(|c| **c == ')').count();
        if matches!(c, '.' | ',' | ';' | ':' | '!' | '?' | '\'' | '"') || (c == ')' && close > open) {
            e -= 1;
        } else {
            break;
        }
    }
    e
}

fn is_url(s: &str) -> bool {
    (s.starts_with("https://") || s.starts_with("http://")) && s.len() > 8 && !s.contains(char::is_whitespace)
}

/// The runs of a line from `from` on, plain ones included, in order.
pub(crate) fn spans(line: &[char], from: usize) -> Vec<Span> {
    let n = line.len();
    let mut out: Vec<Span> = Vec::new();
    let push = |out: &mut Vec<Span>, s: usize, e: usize, look: Look, url: Option<String>| {
        if e > s {
            out.push(Span { s, e, look, url });
        }
    };
    let mark = Look { mark: true, ..Default::default() };
    let (mut i, mut plain) = (from, from);
    while i < n {
        // **bold** and __underline__
        let pair = if starts(line, i, "**") {
            Some(("**", Look { bold: true, ..Default::default() }))
        } else if starts(line, i, "__") {
            Some(("__", Look { under: true, ..Default::default() }))
        } else {
            None
        };
        if let Some((m, look)) = pair {
            // (the words inside start and end right by the marks, so "2 ** 3" stays as written)
            let tight = i + 2 < n && !line[i + 2].is_whitespace();
            let close = (i + 3..n.saturating_sub(1)).find(|&j| starts(line, j, m) && !line[j - 1].is_whitespace());
            if let Some(j) = close.filter(|_| tight) {
                push(&mut out, plain, i, Look::default(), None);
                push(&mut out, i, i + 2, mark, None);
                push(&mut out, i + 2, j, look, None);
                push(&mut out, j, j + 2, mark, None);
                i = j + 2;
                plain = i;
                continue;
            }
        }
        // [name](address)
        if line[i] == '[' {
            if let Some(k) = (i + 1..n).find(|&k| line[k] == ']') {
                if k + 1 < n && line[k + 1] == '(' {
                    let mut depth = 0;
                    let mut close = None;
                    for (m, &c) in line.iter().enumerate().skip(k + 2) {
                        if c == '(' {
                            depth += 1;
                        } else if c == ')' {
                            if depth == 0 {
                                close = Some(m);
                                break;
                            }
                            depth -= 1;
                        }
                    }
                    if let Some(m) = close {
                        let url: String = line[k + 2..m].iter().collect();
                        if is_url(&url) {
                            push(&mut out, plain, i, Look::default(), None);
                            push(&mut out, i, i + 1, mark, None);
                            if k > i + 1 {
                                push(&mut out, i + 1, k, Look { link: true, ..Default::default() }, Some(url));
                                push(&mut out, k, m + 1, mark, None);
                                i = m + 1;
                                plain = i;
                                continue;
                            } else {
                                // no name: the address itself is the link
                                push(&mut out, k, k + 2, mark, None);
                                push(&mut out, k + 2, m, Look { link: true, ..Default::default() }, Some(url));
                                push(&mut out, m, m + 1, mark, None);
                                i = m + 1;
                                plain = i;
                                continue;
                            }
                        }
                    }
                }
            }
        }
        // a bare web address, at the start of a word
        if (starts(line, i, "https://") || starts(line, i, "http://")) && (i == 0 || line[i - 1].is_whitespace() || line[i - 1] == '(') {
            let e = url_end(line, i);
            let url: String = line[i..e].iter().collect();
            if is_url(&url) {
                push(&mut out, plain, i, Look::default(), None);
                push(&mut out, i, e, Look { link: true, ..Default::default() }, Some(url));
                i = e;
                plain = i;
                continue;
            }
        }
        i += 1;
    }
    push(&mut out, plain, n, Look::default(), None);
    out
}

/// How each character of a text is shown (one per char).
pub(crate) fn looks(chars: &[char]) -> Vec<Look> {
    let mut out = vec![Look::default(); chars.len()];
    let mut start = 0;
    for end in (0..=chars.len()).filter(|&k| k == chars.len() || chars[k] == '\n') {
        let line = &chars[start..end];
        let (block, from) = block_of(line);
        let head = block == Block::Heading;
        for l in out[start..start + from].iter_mut() {
            l.mark = true;
        }
        if block != Block::Rule {
            for sp in spans(line, from) {
                for l in out[start + sp.s..start + sp.e].iter_mut() {
                    *l = Look { head, ..sp.look };
                }
            }
        }
        start = end + 1;
    }
    out
}

/// A text without its marks, on one line: for a preview.
pub(crate) fn plain(text: &str) -> String {
    let mut words: Vec<String> = Vec::new();
    for raw in text.lines() {
        let line: Vec<char> = raw.chars().collect();
        let (block, from) = block_of(&line);
        if block == Block::Rule {
            continue;
        }
        let mut s = String::new();
        for sp in spans(&line, from) {
            if !sp.look.mark {
                s.extend(&line[sp.s..sp.e]);
            }
        }
        words.extend(s.split_whitespace().map(str::to_string));
    }
    words.join(" ")
}

/// The formatting tools.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Tool {
    Heading,
    Bold,
    Underline,
    List,
    Link,
    Line,
}

/// Each tool and what it does, for its button.
pub(crate) const TOOLS: [(Tool, &str); 6] = [
    (Tool::Heading, "Heading: starts a section."),
    (Tool::Bold, "Bold (Ctrl+B)."),
    (Tool::Underline, "Underline (Ctrl+U)."),
    (Tool::List, "List: a dot at the start of each line."),
    (Tool::Link, "Link (Ctrl+K): paste the web address over https://."),
    (Tool::Line, "A line between sections."),
];

/// A tool used on `text` with the selection `sel` (chars; the caret alone when both ends are the same): the new text,
/// and what is selected after (the words wrapped, or where to type or paste next).
pub(crate) fn apply(text: &str, sel: (usize, usize), tool: Tool) -> (String, (usize, usize)) {
    let mut c: Vec<char> = text.chars().collect();
    let n = c.len();
    let (s, e) = (sel.0.min(sel.1).min(n), sel.0.max(sel.1).min(n));
    let ins = |c: &mut Vec<char>, at: usize, t: &str| {
        for (k, ch) in t.chars().enumerate() {
            c.insert(at + k, ch);
        }
    };
    // the lines the selection touches: from the start of the first to the end of the last
    let ls = c[..s].iter().rposition(|&ch| ch == '\n').map_or(0, |k| k + 1);
    let le = c[e..].iter().position(|&ch| ch == '\n').map_or(n, |k| e + k);
    match tool {
        Tool::Bold | Tool::Underline => {
            let m = if tool == Tool::Bold { "**" } else { "__" };
            let mc: Vec<char> = m.chars().collect();
            if s >= 2 && c[s - 2..s] == mc[..] && e + 2 <= n && c[e..e + 2] == mc[..] {
                // already that way: undone
                c.drain(e..e + 2);
                c.drain(s - 2..s);
                (c.iter().collect(), (s - 2, e - 2))
            } else if e >= s + 4 && c[s..s + 2] == mc[..] && c[e - 2..e] == mc[..] {
                c.drain(e - 2..e);
                c.drain(s..s + 2);
                (c.iter().collect(), (s, e - 4))
            } else {
                ins(&mut c, e, m);
                ins(&mut c, s, m);
                (c.iter().collect(), (s + 2, e + 2))
            }
        }
        Tool::Heading | Tool::List => {
            let (pre, other) = if tool == Tool::Heading { ("# ", "- ") } else { ("- ", "# ") };
            let block: String = c[ls..le].iter().collect();
            let lines: Vec<&str> = block.split('\n').collect();
            let filled: Vec<&&str> = lines.iter().filter(|l| !l.trim().is_empty()).collect();
            let on = !filled.is_empty() && filled.iter().all(|l| l.starts_with(pre));
            let one = lines.len() == 1;
            let new: Vec<String> = lines
                .iter()
                .map(|l| {
                    if on {
                        l.strip_prefix(pre).unwrap_or(l).to_string()
                    } else if l.trim().is_empty() && !one {
                        l.to_string()
                    } else if l.starts_with(pre) {
                        l.to_string()
                    } else {
                        format!("{}{}", pre, l.strip_prefix(other).unwrap_or(l))
                    }
                })
                .collect();
            let first = new[0].chars().count() as isize - lines[0].chars().count() as isize;
            let joined = new.join("\n");
            let total = joined.chars().count() as isize - (le - ls) as isize;
            let out: String = c[..ls].iter().chain(joined.chars().collect::<Vec<_>>().iter()).chain(c[le..].iter()).collect();
            let ns = ((s as isize + first).max(ls as isize)) as usize;
            let ne = ((e as isize + total).max(ns as isize)) as usize;
            (out, (ns, ne))
        }
        Tool::Link => {
            let picked: String = c[s..e].iter().collect();
            if is_url(picked.trim()) {
                // an address picked: it gets a name to type over
                let t = format!("[link]({})", picked.trim());
                c.splice(s..e, t.chars()).for_each(drop);
                (c.iter().collect(), (s + 1, s + 5))
            } else {
                let name = if picked.is_empty() { "link".to_string() } else { picked };
                let t = format!("[{}](https://)", name);
                c.splice(s..e, t.chars()).for_each(drop);
                let at = s + 1 + name.chars().count() + 2;
                (c.iter().collect(), (at, at + 8))
            }
        }
        Tool::Line => {
            // on a line of its own, after the line the caret is in
            let line_end = c[e..].iter().position(|&ch| ch == '\n').map_or(n, |k| e + k);
            let blank = c[ls..line_end].iter().all(|ch| ch.is_whitespace());
            let t = if blank { "---\n" } else { "\n---\n" };
            let at = if blank { ls } else { line_end };
            if c.get(line_end) == Some(&'\n') {
                c.remove(line_end);
            }
            ins(&mut c, at, t);
            let to = at + t.chars().count();
            (c.iter().collect(), (to, to))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shown(text: &str) -> Vec<(String, Look)> {
        let c: Vec<char> = text.chars().collect();
        spans(&c, 0).into_iter().map(|s| (c[s.s..s.e].iter().collect(), s.look)).collect()
    }

    #[test]
    fn the_marks_are_read() {
        let bold = Look { bold: true, ..Default::default() };
        let mark = Look { mark: true, ..Default::default() };
        let got = shown("The **Bbm7b5** chord");
        assert_eq!(got[1], ("**".to_string(), mark));
        assert_eq!(got[2], ("Bbm7b5".to_string(), bold));
        assert_eq!(got.len(), 5);
        // unclosed or empty marks are just text
        assert!(shown("2 ** 3 and ****").iter().all(|s| s.1 == Look::default()));
        // a link with brackets in its address, and a bare address at the end of a sentence
        let c: Vec<char> = "See [the tune](https://en.wikipedia.org/wiki/Autumn_Leaves_(1945_song)) or https://example.com.".chars().collect();
        let urls: Vec<String> = spans(&c, 0).into_iter().filter_map(|s| s.url.filter(|_| s.look.link)).collect();
        assert_eq!(urls, ["https://en.wikipedia.org/wiki/Autumn_Leaves_(1945_song)", "https://example.com"]);
        // not a web address: left alone
        assert!(shown("[x](notes.txt)").iter().all(|s| !s.1.link));
        assert_eq!(plain("# Form\nAABC, __bar 13__ is it.\n---\n- [Jim Hall](https://x.org/a)"), "Form AABC, bar 13 is it. Jim Hall");
        let lk = looks(&"# Hi **there**\n---".chars().collect::<Vec<_>>());
        assert!(lk[0].mark && lk[1].mark && lk[2].head && !lk[2].bold && lk[5].mark && lk[6].mark && lk[7].bold && lk[7].head);
        assert!(lk[15..].iter().all(|l| l.mark));
    }

    #[test]
    fn the_tools_put_marks_in_and_take_them_out() {
        let t = "slow it down";
        let (b, sel) = apply(t, (5, 7), Tool::Bold);
        assert_eq!((b.as_str(), sel), ("slow **it** down", (7, 9)));
        assert_eq!(apply(&b, sel, Tool::Bold), (t.to_string(), (5, 7)));
        // the caret alone: marks to type between
        assert_eq!(apply("ab", (1, 1), Tool::Underline), ("a____b".to_string(), (3, 3)));
        // headings and lists by the line, one taking over from the other
        let (h, _) = apply("one\ntwo", (5, 5), Tool::Heading);
        assert_eq!(h, "one\n# two");
        let (l, sel) = apply(&h, (0, 9), Tool::List);
        assert_eq!((l.as_str(), sel), ("- one\n- two", (2, 11)));
        assert_eq!(apply(&l, (0, 11), Tool::List).0, "one\ntwo");
        // a link: the address to paste over is picked
        let (k, sel) = apply("Jim Hall", (0, 8), Tool::Link);
        assert_eq!(k, "[Jim Hall](https://)");
        assert_eq!(k.chars().skip(sel.0).take(sel.1 - sel.0).collect::<String>(), "https://");
        let (k, sel) = apply("https://a.org/x", (0, 15), Tool::Link);
        assert_eq!((k.as_str(), sel), ("[link](https://a.org/x)", (1, 5)));
        // a line between sections, on its own line
        assert_eq!(apply("abc\ndef", (1, 1), Tool::Line), ("abc\n---\ndef".to_string(), (8, 8)));
        assert_eq!(apply("abc", (3, 3), Tool::Line), ("abc\n---\n".to_string(), (8, 8)));
        assert_eq!(apply("", (0, 0), Tool::Line), ("---\n".to_string(), (4, 4)));
    }
}
