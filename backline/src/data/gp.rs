//! Guitar Pro files (.gp, version 7 and on: a zip with the score as XML in `Content/score.gpif`): the tune's bars,
//! every note with its string and fret, the rhythm, the chord names and their grids, and the recording when the
//! file carries one. Read for TRANSCRIPTIONS; the files stay where they are on this computer.

use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::HashMap;
use std::io::Read;

/// One note: its pitch, and where it is played (string 0 = low E, fret).
#[derive(Clone, Debug, PartialEq)]
pub struct GpNote {
    pub midi: i32,
    pub string: usize,
    pub fret: u8,
    /// tied over from the note before (not struck again)
    pub tied: bool,
}

/// What sounds at one moment: its start and length in quarter notes from the top of the bar, the notes struck,
/// and the chord name written over it (if any).
#[derive(Clone, Debug)]
pub struct GpBeat {
    pub start: f32,
    pub len: f32,
    pub notes: Vec<GpNote>,
    pub chord: Option<String>,
}

#[derive(Clone, Debug)]
pub struct GpBar {
    /// beats a bar and the note that gets the beat (4/4, 3/4, 6/8...)
    pub time: (u32, u32),
    /// the key signature: sharps (+) or flats (-)
    pub key: i32,
    /// every voice's beats, merged and in time order
    pub beats: Vec<GpBeat>,
}

/// A chord as the file draws it: its name and its grid (fret per string, low E first; None not played).
#[derive(Clone, Debug)]
pub struct GpChord {
    pub name: String,
    pub frets: [Option<u8>; 6],
}

#[derive(Clone, Debug, Default)]
pub struct GpSong {
    pub title: String,
    pub artist: String,
    /// quarter notes a minute at the start
    pub tempo: f32,
    pub bars: Vec<GpBar>,
    pub chords: Vec<GpChord>,
    /// the recording inside the file (mp3 bytes), when there is one
    pub audio: Option<Vec<u8>>,
}

/// A small XML tree: enough to walk the score.
#[derive(Debug, Default)]
struct Node {
    name: String,
    attrs: Vec<(String, String)>,
    text: String,
    kids: Vec<Node>,
}

impl Node {
    fn kid(&self, name: &str) -> Option<&Node> {
        self.kids.iter().find(|k| k.name == name)
    }
    fn kids<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Node> + 'a {
        self.kids.iter().filter(move |k| k.name == name)
    }
    fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.iter().find(|a| a.0 == name).map(|a| a.1.as_str())
    }
    /// The text at a path of child names.
    fn at(&self, path: &[&str]) -> Option<&str> {
        let mut n = self;
        for p in path {
            n = n.kid(p)?;
        }
        Some(n.text.trim())
    }
    /// The first `<Property name="...">` anywhere below this node.
    fn find_prop(&self, name: &str) -> Option<&Node> {
        if self.name == "Property" && self.attr("name") == Some(name) {
            return Some(self);
        }
        self.kids.iter().find_map(|k| k.find_prop(name))
    }

    /// A `<Property name="...">` child.
    fn prop(&self, name: &str) -> Option<&Node> {
        self.kid("Properties")?.kids("Property").find(|p| p.attr("name") == Some(name))
    }
}

fn parse_xml(text: &str) -> Result<Node, String> {
    let mut r = Reader::from_str(text);
    let mut stack: Vec<Node> = vec![Node::default()];
    let attrs_of = |e: &quick_xml::events::BytesStart| -> Vec<(String, String)> {
        e.attributes()
            .flatten()
            .map(|a| {
                (
                    String::from_utf8_lossy(a.key.as_ref()).to_string(),
                    a.unescape_value().map(|v| v.to_string()).unwrap_or_default(),
                )
            })
            .collect()
    };
    loop {
        match r.read_event().map_err(|e| e.to_string())? {
            Event::Start(e) => stack.push(Node {
                name: String::from_utf8_lossy(e.name().as_ref()).to_string(),
                attrs: attrs_of(&e),
                ..Default::default()
            }),
            Event::Empty(e) => {
                let n = Node {
                    name: String::from_utf8_lossy(e.name().as_ref()).to_string(),
                    attrs: attrs_of(&e),
                    ..Default::default()
                };
                if let Some(top) = stack.last_mut() {
                    top.kids.push(n);
                }
            }
            Event::Text(t) => {
                if let Some(top) = stack.last_mut() {
                    top.text.push_str(&t.unescape().map(|s| s.to_string()).unwrap_or_default());
                }
            }
            Event::CData(t) => {
                if let Some(top) = stack.last_mut() {
                    top.text.push_str(&String::from_utf8_lossy(&t));
                }
            }
            Event::End(_) => {
                let done = stack.pop().ok_or("bad XML")?;
                stack.last_mut().ok_or("bad XML")?.kids.push(done);
            }
            Event::Eof => break,
            _ => {}
        }
    }
    stack.pop().ok_or_else(|| "empty XML".to_string())
}

/// A rhythm's length in quarter notes: its note value, dots and tuplet.
fn rhythm_len(r: &Node) -> f32 {
    let base = match r.at(&["NoteValue"]).unwrap_or("Quarter") {
        "Whole" => 4.0,
        "Half" => 2.0,
        "Quarter" => 1.0,
        "Eighth" => 0.5,
        "16th" => 0.25,
        "32nd" => 0.125,
        "64th" => 0.0625,
        "128th" => 0.03125,
        _ => 1.0,
    };
    let dots = r.kid("AugmentationDot").and_then(|d| d.attr("count")).and_then(|c| c.parse::<i32>().ok()).unwrap_or(0);
    let mut len = base;
    let mut add = base / 2.0;
    for _ in 0..dots {
        len += add;
        add /= 2.0;
    }
    if let Some(t) = r.kid("PrimaryTuplet") {
        let num = t.attr("num").and_then(|v| v.parse::<f32>().ok()).unwrap_or(1.0);
        let den = t.attr("den").and_then(|v| v.parse::<f32>().ok()).unwrap_or(1.0);
        if num > 0.0 {
            len *= den / num;
        }
    }
    len
}

/// Ids in a space-separated list ("0 1 2"; -1 for none).
fn ids(s: Option<&str>) -> Vec<String> {
    s.unwrap_or("").split_whitespace().filter(|x| *x != "-1").map(|x| x.to_string()).collect()
}

/// A list's items by their id.
fn table<'a>(gpif: &'a Node, list: &str, item: &'a str) -> HashMap<String, &'a Node> {
    gpif.kid(list).map(|l| l.kids(item).filter_map(|n| n.attr("id").map(|id| (id.to_string(), n))).collect()).unwrap_or_default()
}

/// Read a Guitar Pro file (the first track: the guitar).
pub fn read(bytes: &[u8]) -> Result<GpSong, String> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|_| "not a Guitar Pro 7+ file (.gp)".to_string())?;
    let mut xml = String::new();
    zip.by_name("Content/score.gpif")
        .map_err(|_| "no score in this file".to_string())?
        .read_to_string(&mut xml)
        .map_err(|e| e.to_string())?;
    // the recording, if the file carries one
    let mut audio = None;
    for i in 0..zip.len() {
        if let Ok(mut f) = zip.by_index(i) {
            let name = f.name().to_lowercase();
            if name.starts_with("content/assets/") && (name.ends_with(".mp3") || name.ends_with(".ogg") || name.ends_with(".wav"))
            {
                let mut b = Vec::new();
                if f.read_to_end(&mut b).is_ok() {
                    audio = Some(b);
                    break;
                }
            }
        }
    }
    let doc = parse_xml(&xml)?;
    let gpif = doc.kid("GPIF").ok_or("no score in this file")?;
    let score = gpif.kid("Score");
    let title = score.and_then(|s| s.at(&["Title"])).unwrap_or("").to_string();
    let artist = score.and_then(|s| s.at(&["Artist"])).unwrap_or("").to_string();
    // the tempo at the top
    let tempo = gpif
        .kid("MasterTrack")
        .and_then(|m| m.kid("Automations"))
        .and_then(|a| a.kids("Automation").find(|x| x.at(&["Type"]) == Some("Tempo")))
        .and_then(|a| a.at(&["Value"]))
        .and_then(|v| v.split_whitespace().next())
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(120.0);

    // the lookup tables, by id
    let bars_t = table(gpif, "Bars", "Bar");
    let voices_t = table(gpif, "Voices", "Voice");
    let beats_t = table(gpif, "Beats", "Beat");
    let notes_t = table(gpif, "Notes", "Note");
    let rhythms_t = table(gpif, "Rhythms", "Rhythm");

    // the guitar's chord names and grids (the first track's diagram collection)
    let track = gpif.kid("Tracks").and_then(|t| t.kids("Track").next());
    let mut chord_names: HashMap<String, String> = HashMap::new();
    let mut chords: Vec<GpChord> = Vec::new();
    if let Some(items) = track
        .and_then(|t| t.find_prop("DiagramCollection").filter(|p| p.kid("Items").is_some_and(|i| !i.kids.is_empty())))
        .and_then(|p| p.kid("Items"))
    {
        for it in items.kids("Item") {
            let (Some(id), Some(name)) = (it.attr("id"), it.attr("name")) else { continue };
            chord_names.insert(id.to_string(), name.to_string());
            if let Some(d) = it.kid("Diagram") {
                let base = d.attr("baseFret").and_then(|b| b.parse::<u8>().ok()).unwrap_or(0);
                let mut frets = [None; 6];
                for f in d.kids("Fret") {
                    let s = f.attr("string").and_then(|v| v.parse::<usize>().ok()).unwrap_or(9);
                    let fr = f.attr("fret").and_then(|v| v.parse::<u8>().ok()).unwrap_or(0);
                    if s < 6 {
                        frets[s] = Some(if fr == 0 { 0 } else { base + fr });
                    }
                }
                chords.push(GpChord { name: name.to_string(), frets });
            }
        }
    }

    let note_of = |id: &str| -> Option<GpNote> {
        let n = notes_t.get(id)?;
        let midi = n.prop("Midi").and_then(|p| p.at(&["Number"])).and_then(|v| v.parse::<i32>().ok())?;
        let string = n.prop("String").and_then(|p| p.at(&["String"])).and_then(|v| v.parse::<usize>().ok()).unwrap_or(0);
        let fret = n.prop("Fret").and_then(|p| p.at(&["Fret"])).and_then(|v| v.parse::<u8>().ok()).unwrap_or(0);
        let tied = n.kid("Tie").and_then(|t| t.attr("destination")) == Some("true");
        Some(GpNote { midi, string: string.min(5), fret, tied })
    };

    let mut bars: Vec<GpBar> = Vec::new();
    if let Some(mbs) = gpif.kid("MasterBars") {
        for mb in mbs.kids("MasterBar") {
            let time = mb
                .at(&["Time"])
                .and_then(|t| t.split_once('/'))
                .and_then(|(a, b)| Some((a.parse().ok()?, b.parse().ok()?)))
                .unwrap_or((4, 4));
            let key = mb.at(&["Key", "AccidentalCount"]).and_then(|k| k.trim().parse().ok()).unwrap_or(0);
            // the first track's bar in this master bar
            let bar_id = ids(mb.at(&["Bars"])).into_iter().next();
            let mut beats: Vec<GpBeat> = Vec::new();
            if let Some(bar) = bar_id.and_then(|b| bars_t.get(&b)) {
                for vid in ids(bar.at(&["Voices"])) {
                    let Some(v) = voices_t.get(&vid) else { continue };
                    let mut at = 0.0f32;
                    for bid in ids(v.at(&["Beats"])) {
                        let Some(b) = beats_t.get(&bid) else { continue };
                        let len = b
                            .kid("Rhythm")
                            .and_then(|r| r.attr("ref"))
                            .and_then(|r| rhythms_t.get(r))
                            .map_or(1.0, |r| rhythm_len(r));
                        let notes: Vec<GpNote> = ids(b.at(&["Notes"])).iter().filter_map(|n| note_of(n)).collect();
                        let chord = b.at(&["Chord"]).and_then(|c| chord_names.get(c).cloned());
                        // a beat at the same moment in another voice joins it
                        if let Some(same) = beats.iter_mut().find(|x| (x.start - at).abs() < 1e-3) {
                            same.notes.extend(notes);
                            same.len = same.len.max(len);
                            if same.chord.is_none() {
                                same.chord = chord;
                            }
                        } else {
                            beats.push(GpBeat { start: at, len, notes, chord });
                        }
                        at += len;
                    }
                }
            }
            beats.sort_by(|a, b| a.start.total_cmp(&b.start));
            bars.push(GpBar { time, key, beats });
        }
    }
    if bars.is_empty() {
        return Err("no bars in this file".to_string());
    }
    Ok(GpSong { title, artist, tempo, bars, chords, audio })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rhythms_add_up() {
        let x = parse_xml("<R><NoteValue>Eighth</NoteValue><AugmentationDot count=\"1\"/></R>").unwrap();
        assert!((rhythm_len(x.kid("R").unwrap()) - 0.75).abs() < 1e-6);
        let t = parse_xml("<R><NoteValue>Eighth</NoteValue><PrimaryTuplet num=\"3\" den=\"2\"/></R>").unwrap();
        assert!((rhythm_len(t.kid("R").unwrap()) - 1.0 / 3.0).abs() < 1e-6);
    }

    /// The owner's own files, when they are on this computer (not part of the normal test run).
    #[test]
    #[ignore]
    fn reads_the_collections() {
        let home = std::env::var("USERPROFILE").unwrap_or_default();
        for dir in ["Joe-Pass-Collection-GP-Files", "Fingerstyle-Jazz-Guitar-Collection-GP-Files-5iydej"] {
            let Ok(rd) = std::fs::read_dir(format!("{}\\Downloads\\{}", home, dir)) else { continue };
            for e in rd.flatten() {
                let b = std::fs::read(e.path()).unwrap();
                match read(&b) {
                    Ok(s) => {
                        let notes: usize = s.bars.iter().map(|b| b.beats.iter().map(|x| x.notes.len()).sum::<usize>()).sum();
                        let named: usize = s.bars.iter().map(|b| b.beats.iter().filter(|x| x.chord.is_some()).count()).sum();
                        eprintln!(
                            "{:55} {:3} bars {:5} notes {:3} chords named {:3} grids  tempo {:5.1}  audio {}",
                            e.file_name().to_string_lossy(),
                            s.bars.len(),
                            notes,
                            named,
                            s.chords.len(),
                            s.tempo,
                            s.audio.as_ref().map_or(0, |a| a.len() / 1024)
                        );
                    }
                    Err(err) => eprintln!("{}: {}", e.file_name().to_string_lossy(), err),
                }
            }
        }
    }
}
