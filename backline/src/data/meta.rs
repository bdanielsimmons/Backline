//! Tempo and key of each track, in one table every view reads. Tidal's own values are used when it sends them;
//! otherwise they are detected from the audio the first time a track plays, and remembered in library.json.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};

#[derive(Clone, Copy, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Info {
    pub bpm: Option<u16>,
    /// 0-11 = C..B major, 12-23 = C..B minor
    pub key: Option<u8>,
    /// came from Tidal (not saved: Tidal sends it again), rather than detected here
    #[serde(skip)]
    pub tidal: bool,
    /// set by you (Key and BPM...): wins over Tidal and detection, kept on this computer only
    pub mine: bool,
    /// the A the recording is tuned to, in Hz (found by FIND, or typed in PITCH); None: not known
    #[serde(skip_serializing_if = "Option::is_none")]
    pub a4: Option<f32>,
}

static TABLE: LazyLock<Mutex<HashMap<i64, Info>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
/// tracks already analyzed without an answer this session, so they are not tried on every play
static TRIED: LazyLock<Mutex<HashSet<i64>>> = LazyLock::new(|| Mutex::new(HashSet::new()));
static BUSY: AtomicBool = AtomicBool::new(false);

pub fn get(id: i64) -> Option<Info> {
    TABLE.lock().unwrap().get(&id).copied()
}

/// What Tidal says about a track wins over a detected guess.
pub fn from_tidal(id: i64, bpm: Option<u16>, key: Option<u8>) {
    let mut t = TABLE.lock().unwrap();
    let e = t.entry(id).or_default();
    if e.mine {
        return;
    }
    e.bpm = bpm.or(e.bpm);
    e.key = key.or(e.key);
    e.tidal = true;
}

/// Fill in what is still missing from a detection.
pub fn set_detected(id: i64, found: Info) {
    let mut t = TABLE.lock().unwrap();
    let e = t.entry(id).or_default();
    e.bpm = e.bpm.or(found.bpm);
    e.key = e.key.or(found.key);
}

/// Your own key and tempo for a track (None for both = forget yours).
pub fn set_mine(id: i64, bpm: Option<u16>, key: Option<u8>) {
    let mut t = TABLE.lock().unwrap();
    // (the record's tuning stays, whatever happens to the key and tempo)
    let a4 = t.get(&id).and_then(|i| i.a4);
    if bpm.is_none() && key.is_none() {
        t.remove(&id);
        TRIED.lock().unwrap().remove(&id);
        if a4.is_some() {
            t.insert(id, Info { a4, ..Default::default() });
        }
    } else {
        t.insert(id, Info { bpm, key, tidal: false, mine: true, a4 });
    }
}

/// The A a recording is tuned to, in Hz (found or typed; None: forget it).
pub fn set_a4(id: i64, hz: Option<f32>) {
    TABLE.lock().unwrap().entry(id).or_default().a4 = hz;
}

/// The A (Hz) of a recording that is `cents` off A440.
pub fn a4_of(cents: f32) -> f32 {
    440.0 * 2f32.powf(cents / 1200.0)
}

/// How far, in cents, to move a recording tuned to A = `from` Hz so its A sounds at `to` Hz.
pub fn cents_between(from: f32, to: f32) -> f32 {
    1200.0 * (to / from).log2()
}

/// An A typed in Hz ("452", "A=442", "440 hz"), within what any record is tuned to.
pub fn parse_hz(s: &str) -> Option<f32> {
    let s = s.trim().to_ascii_lowercase().replace(' ', "");
    let s = s.trim_start_matches("a=").trim_start_matches("a4=").trim_end_matches("hz");
    s.parse::<f32>().ok().filter(|h| (380.0..=500.0).contains(h))
}

/// A key typed by hand: "A", "F#m", "Eb minor", "Bb major".
pub fn parse_typed_key(s: &str) -> Option<u8> {
    let s = s.trim();
    let lower = s.to_ascii_lowercase();
    let minor = lower.contains("min") || (s.ends_with('m') && !lower.ends_with("maj"));
    let two = s.chars().nth(1).is_some_and(|c| c == '#' || c == 'b');
    let root: String = s.chars().take(if two { 2 } else { 1 }).collect();
    parse_key(&root, if minor { "MINOR" } else { "MAJOR" })
}

/// Start of the session: the detected values saved last time.
pub fn load(saved: &HashMap<i64, Info>) {
    TABLE.lock().unwrap().extend(saved.iter().map(|(k, v)| (*k, *v)));
}

/// The detected values, for library.json.
pub fn saved() -> HashMap<i64, Info> {
    TABLE.lock().unwrap().iter().filter(|(_, v)| !v.tidal || v.mine || v.a4.is_some()).map(|(k, v)| (*k, *v)).collect()
}

/// Claim the analyzer for this track, if it still lacks tempo or key and nothing else is being analyzed.
pub fn claim(id: i64) -> bool {
    let known = get(id).is_some_and(|i| i.bpm.is_some() && i.key.is_some());
    if known || TRIED.lock().unwrap().contains(&id) {
        return false;
    }
    !BUSY.swap(true, Ordering::AcqRel)
}

/// Tempo and key from a stored track's audio (a second or two of work, in the background).
pub fn detect(id: i64, bytes: Vec<u8>) -> Info {
    let bpm = crate::band::detect_beats(bytes.clone()).map(|(period, _, _)| (60.0f32 / period).round() as u16);
    let key = crate::tuning::decode_mono(bytes, 155.0).and_then(|(x, sr)| {
        let cents = crate::tuning::offset_cents(&x, sr).filter(|c| c.1 >= 0.04).map_or(0, |c| c.0);
        crate::tuning::key_of(&x, sr, cents).map(|k| k.0)
    });
    if bpm.is_none() || key.is_none() {
        TRIED.lock().unwrap().insert(id);
    }
    BUSY.store(false, Ordering::Release);
    Info { bpm, key, ..Default::default() }
}

const MAJOR: [&str; 12] = ["C", "Db", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"];
const MINOR: [&str; 12] = ["Cm", "C#m", "Dm", "Ebm", "Em", "Fm", "F#m", "Gm", "G#m", "Am", "Bbm", "Bm"];

/// Short name: "F#", "Bbm".
pub fn key_short(k: u8) -> &'static str {
    if k < 12 {
        MAJOR[k as usize]
    } else {
        MINOR[(k % 12) as usize]
    }
}

/// What a list or the player shows: Tidal's key as is, a detected one with a "?" (it is an estimate).
pub fn key_text(i: &Info) -> String {
    i.key.map_or(String::new(), |k| format!("{}{}", key_short(k), if i.tidal || i.mine { "" } else { "?" }))
}

/// Long name with the Camelot code DJs mix by: "A minor (8A)".
pub fn key_long(k: u8) -> String {
    let pc = k as usize % 12;
    let minor = k >= 12;
    // the Camelot wheel steps by fifths; C major is 8B and its relative minor, A minor, is 8A
    let major_pc = if minor { (pc + 3) % 12 } else { pc };
    let num = (major_pc * 7 + 7) % 12 + 1;
    let name = if minor { MINOR[pc].trim_end_matches('m') } else { MAJOR[pc] };
    format!("{} {} ({}{})", name, if minor { "minor" } else { "major" }, num, if minor { 'A' } else { 'B' })
}

/// Tidal's key ("A", "FSharp", "Eb"...) and scale ("MAJOR", "MINOR"...) as a key number.
pub fn parse_key(key: &str, scale: &str) -> Option<u8> {
    let k = key.trim().replace("Sharp", "#").replace("Flat", "b");
    let mut ch = k.chars();
    let base = match ch.next()?.to_ascii_uppercase() {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    };
    let rest: String = ch.collect();
    let pc = match rest.chars().next() {
        Some('#') => (base + 1) % 12,
        Some('b') => (base + 11) % 12,
        _ => base,
    };
    let scale = scale.to_ascii_uppercase();
    let minor = if scale.contains("MINOR") || scale.contains("AEOLIAN") || rest.ends_with('m') {
        true
    } else if scale.contains("MAJOR") || scale.contains("IONIAN") {
        false
    } else {
        return None; // a note without major or minor is not a key
    };
    Some(pc as u8 + if minor { 12 } else { 0 })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tuning_in_hz_and_cents() {
        // a record at A = 452: down 46.6 cents to play along at 440
        assert!((cents_between(452.0, 440.0) + 46.6).abs() < 0.1);
        assert!((a4_of(46.6) - 452.0).abs() < 0.1);
        assert!((a4_of(0.0) - 440.0).abs() < 1e-3);
        assert!((cents_between(440.0, 442.0) - 7.85).abs() < 0.05, "an orchestra's 442 is 8 cents up");
        assert_eq!(parse_hz("452"), Some(452.0));
        assert_eq!(parse_hz("A=442"), Some(442.0));
        assert_eq!(parse_hz(" 432 Hz"), Some(432.0));
        assert_eq!(parse_hz("44"), None, "not an A anyone tunes to");
        // the record's A stays when the key and tempo are set or forgotten
        set_a4(777001, Some(452.0));
        set_mine(777001, Some(120), None);
        assert_eq!(get(777001).and_then(|i| i.a4), Some(452.0));
        set_mine(777001, None, None);
        assert_eq!(get(777001).and_then(|i| i.a4), Some(452.0));
    }

    #[test]
    fn typed_keys() {
        assert_eq!(parse_typed_key("A"), Some(9));
        assert_eq!(parse_typed_key("F#m"), Some(18));
        assert_eq!(parse_typed_key("Eb minor"), Some(15));
        assert_eq!(parse_typed_key("Bb major"), Some(10));
        assert_eq!(parse_typed_key("x"), None);
    }

    #[test]
    fn names_and_camelot() {
        assert_eq!(key_long(0), "C major (8B)");
        assert_eq!(key_long(12 + 9), "A minor (8A)");
        assert_eq!(key_long(7), "G major (9B)");
        assert_eq!(key_long(5), "F major (7B)");
        assert_eq!(key_long(12 + 4), "E minor (9A)");
        assert_eq!(key_long(12 + 3), "Eb minor (2A)");
        assert_eq!(key_short(6), "F#");
        assert_eq!(key_short(12 + 10), "Bbm");
    }

    #[test]
    fn reads_tidal_keys() {
        assert_eq!(parse_key("FSharp", "MAJOR"), Some(6));
        assert_eq!(parse_key("Eb", "MINOR"), Some(12 + 3));
        assert_eq!(parse_key("A", "Minor"), Some(12 + 9));
        assert_eq!(parse_key("C", "UNKNOWN"), None);
        assert_eq!(parse_key("UNKNOWN", "MAJOR"), None);
    }
}
