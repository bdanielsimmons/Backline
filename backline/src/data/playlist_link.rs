//! A playlist from another app: an Apple Music or Spotify link. Its songs are read from the page anyone can open (no
//! account), then each is found on Tidal (logged in) or on YouTube (not), and the New Playlist panel opens with them
//! (`Action::PlaylistImport`, `Msg::PlaylistImported`).

use crate::api::{Api, Track};
use crate::search::{edits, fold};
use crate::store::Ext;
use serde_json::Value;
use std::time::Duration;

/// A song as the other app lists it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Song {
    pub title: String,
    /// everyone credited, as named there
    pub artists: Vec<String>,
    /// its album (Apple Music says; Spotify's shared page doesn't)
    pub album: String,
    /// seconds (0: not said)
    pub dur: f32,
}

impl Song {
    /// "Title - Artist", for the songs not found.
    pub fn label(&self) -> String {
        match self.artists.first() {
            Some(a) => format!("{} - {}", self.title, a),
            None => self.title.clone(),
        }
    }
}

/// What an import found: the playlist's name, the app it came from, the songs found (in order), the YouTube clips
/// among them (so they're kept as clips), the songs not found, and how many the playlist has in all.
#[derive(Default)]
pub struct Imported {
    pub name: String,
    pub from: &'static str,
    pub tracks: Vec<Track>,
    pub clips: Vec<Ext>,
    pub missing: Vec<String>,
    pub total: usize,
}

/// Which app a link is from (None: not a playlist link this reads).
pub fn source(url: &str) -> Option<&'static str> {
    let u = url.trim().to_lowercase();
    if u.contains("music.apple.com/") && u.contains("/playlist/") {
        Some("Apple Music")
    } else if (u.contains("open.spotify.com/") && u.contains("playlist/")) || u.starts_with("spotify:playlist:") {
        Some("Spotify")
    } else {
        None
    }
}

/// A Spotify playlist's id, from its link or its "spotify:playlist:" name.
fn spotify_id(url: &str) -> Option<String> {
    let u = url.trim();
    let at = u.find("playlist/").map(|i| i + 9).or_else(|| u.find("playlist:").map(|i| i + 9))?;
    let id: String = u[at..].chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
    Some(id).filter(|s| !s.is_empty())
}

/// The page, as a browser gets it.
fn page(url: &str) -> Result<String, String> {
    crate::api::guard()?;
    let c = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0 Safari/537.36")
        .build()
        .map_err(|e| e.to_string())?;
    let r = c.get(url).send().map_err(|e| e.to_string())?;
    if !r.status().is_success() {
        return Err(format!("the page answered {}", r.status().as_u16()));
    }
    r.text().map_err(|e| e.to_string())
}

/// The JSON in a page's `<script ... id="..." ...>` block.
fn script_json(html: &str, id: &str) -> Option<Value> {
    let at = html.find(&format!("id=\"{}\"", id))?;
    let start = at + html[at..].find('>')? + 1;
    let end = start + html[start..].find("</script>")?;
    serde_json::from_str(&html[start..end]).ok()
}

/// A `<meta property="..." content="...">` value, its &amp; and the like turned back.
fn meta(html: &str, prop: &str) -> Option<String> {
    let at = html.find(&format!("property=\"{}\"", prop))?;
    let tag_start = html[..at].rfind('<')?;
    let tag = &html[tag_start..tag_start + html[tag_start..].find('>')?];
    let c = tag.find("content=\"")? + 9;
    let v = &tag[c..c + tag[c..].find('"')?];
    Some(v.replace("&amp;", "&").replace("&quot;", "\"").replace("&#39;", "'").replace("&apos;", "'").replace("&lt;", "<").replace("&gt;", ">"))
}

/// "A & B", "A, B": each name.
fn split_artists(s: &str) -> Vec<String> {
    s.replace('\u{a0}', " ")
        .split([',', '&'])
        .map(|a| a.trim().to_string())
        .filter(|a| !a.is_empty())
        .collect()
}

/// An Apple Music playlist page: its name, its songs in order, and how many it has in all (a long one's page holds
/// only the first ones).
pub fn apple_songs(html: &str) -> Option<(String, Vec<Song>, usize)> {
    let v = script_json(html, "serialized-server-data")?;
    let is_song = |o: &Value| {
        o["contentDescriptor"]["kind"].as_str() == Some("song") && o["title"].is_string() && o["artistName"].is_string()
    };
    // the list with the most songs in it is the playlist (the page has other shelves)
    fn lists<'a>(v: &'a Value, out: &mut Vec<&'a Vec<Value>>) {
        match v {
            Value::Array(a) => {
                out.push(a);
                a.iter().for_each(|x| lists(x, out));
            }
            Value::Object(o) => o.values().for_each(|x| lists(x, out)),
            _ => {}
        }
    }
    let mut all = Vec::new();
    lists(&v, &mut all);
    let list = all.into_iter().max_by_key(|a| a.iter().filter(|o| is_song(o)).count())?;
    let songs: Vec<Song> = list
        .iter()
        .filter(|o| is_song(o))
        .map(|o| Song {
            title: o["title"].as_str().unwrap_or("").to_string(),
            artists: split_artists(o["artistName"].as_str().unwrap_or("")),
            album: o["tertiaryLinks"][0]["title"].as_str().unwrap_or("").to_string(),
            dur: o["duration"].as_f64().unwrap_or(0.0) as f32 / 1000.0,
        })
        .collect();
    // "Name by Someone on Apple Music"
    let name = meta(html, "og:title").unwrap_or_default();
    let name = name.trim_end_matches(" on Apple Music");
    let name = name.rsplit_once(" by ").map(|(n, _)| n).unwrap_or(name).trim().to_string();
    let total = html
        .find("\"trackCount\":")
        .and_then(|i| html[i + 13..].split(|c: char| !c.is_ascii_digit()).next()?.parse().ok())
        .unwrap_or(songs.len())
        .max(songs.len());
    Some((if name.is_empty() { "Apple Music playlist".to_string() } else { name }, songs, total))
}

/// Spotify's shared player page for a playlist: its name and songs (the first 50 to 100; it has no albums).
pub fn spotify_songs(html: &str) -> Option<(String, Vec<Song>, usize)> {
    let v = script_json(html, "__NEXT_DATA__")?;
    let e = &v["props"]["pageProps"]["state"]["data"]["entity"];
    let songs: Vec<Song> = e["trackList"]
        .as_array()?
        .iter()
        .filter_map(|t| {
            Some(Song {
                title: t["title"].as_str()?.to_string(),
                artists: t["subtitle"].as_str().map(|s| s.replace('\u{a0}', " ").split(',').map(|a| a.trim().to_string()).filter(|a| !a.is_empty()).collect()).unwrap_or_default(),
                album: String::new(),
                dur: t["duration"].as_f64().unwrap_or(0.0) as f32 / 1000.0,
            })
        })
        .collect();
    let name = e["name"].as_str().or_else(|| e["title"].as_str()).unwrap_or("Spotify playlist").to_string();
    let n = songs.len();
    Some((name, songs, n))
}

/// A title for matching: folded, without what's in brackets ("(feat. X)", "[Live]") or after " - " ("- 2011 Remaster").
fn core(title: &str) -> String {
    let mut s = String::new();
    let mut depth = 0u32;
    for c in title.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            _ if depth == 0 => s.push(c),
            _ => {}
        }
    }
    let s = s.split(" - ").next().unwrap_or("").to_string();
    let f = fold(&s).split_whitespace().collect::<Vec<_>>().join(" ");
    if f.is_empty() {
        fold(title).split_whitespace().collect::<Vec<_>>().join(" ")
    } else {
        f
    }
}

/// A live recording, by its title.
fn is_live(title: &str) -> bool {
    fold(title).split_whitespace().any(|w| w == "live")
}

/// The words of names that tell one act from another ("Rev." and "Choir" don't).
fn name_words(names: &str) -> Vec<String> {
    const SKIP: [&str; 20] = [
        "the", "and", "feat", "featuring", "with", "rev", "choir", "mass", "band", "orchestra", "singers", "trio", "quartet",
        "quintet", "ensemble", "group", "live", "his", "her", "jr",
    ];
    fold(names).split_whitespace().filter(|w| w.chars().count() >= 3 && !SKIP.contains(w)).map(|w| w.to_string()).collect()
}

/// How close a Tidal song is to the one in the list (lower is closer; None: not the same song). The same title and
/// someone credited on both are needed; the length, live or not, and the album weigh in.
pub fn closeness(song: &Song, t: &Track) -> Option<i32> {
    let (a, b) = (core(&song.title), core(&t.title));
    let mut s: i32 = if a == b {
        0
    } else if !a.is_empty() && !b.is_empty() && (a.contains(&b) || b.contains(&a)) {
        2
    } else {
        let (ac, bc): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
        let max = (ac.len() / 6).max(1).min(3);
        if edits(&ac, &bc, max) <= max {
            2
        } else {
            return None;
        }
    };
    if !song.artists.is_empty() {
        let theirs = name_words(&song.artists.join(" "));
        let ours = name_words(&format!("{} {}", t.artist, t.artists.iter().map(|a| a.1.as_str()).collect::<Vec<_>>().join(" ")));
        if !theirs.is_empty() && !theirs.iter().any(|w| ours.contains(w)) {
            return None;
        }
    }
    if is_live(&song.title) != is_live(&t.title) {
        s += 2;
    }
    if song.dur > 0.0 && t.duration > 0.0 {
        let d = (song.dur - t.duration).abs();
        s += if d <= 3.0 {
            0
        } else if d <= 10.0 {
            1
        } else if d <= 30.0 {
            3
        } else {
            6
        };
    }
    if !song.album.is_empty() {
        let (x, y) = (core(&song.album), core(&t.album));
        if x == y {
            s -= 2;
        } else if !y.is_empty() && (x.contains(&y) || y.contains(&x)) {
            s -= 1;
        }
    }
    if t.gone {
        s += 4;
    }
    Some(s)
}

/// The closest of these (and how close).
fn best(song: &Song, ts: &[Track]) -> Option<(Track, i32)> {
    ts.iter().filter_map(|t| Some((t.clone(), closeness(song, t)?))).min_by_key(|x| x.1)
}

/// The song on Tidal: searched by title and artist; then on its album (a search can miss a song an album has); then
/// by title alone.
fn on_tidal(api: &Api, song: &Song) -> Option<Track> {
    let title = core(&song.title);
    let artist = song.artists.first().cloned().unwrap_or_default();
    let mut found = api.search_tracks(&format!("{} {}", title, artist), 10).ok().and_then(|ts| best(song, &ts));
    if found.as_ref().is_none_or(|f| f.1 > 1) && !song.album.is_empty() && !artist.is_empty() {
        if let Some(b) = api.find_album(&artist, &song.album).ok().and_then(|ts| best(song, &ts)) {
            if found.as_ref().is_none_or(|f| b.1 < f.1) {
                found = Some(b);
            }
        }
    }
    if found.as_ref().is_none_or(|f| f.1 > 3) {
        if let Some(b) = api.search_tracks(&title, 25).ok().and_then(|ts| best(song, &ts)) {
            if found.as_ref().is_none_or(|f| b.1 < f.1) {
                found = Some(b);
            }
        }
    }
    found.map(|f| f.0)
}

/// The song on YouTube: a clip with its title in the name, the nearest in length; else the first one.
fn on_youtube(song: &Song) -> Option<Ext> {
    let artist = song.artists.first().cloned().unwrap_or_default();
    let list = crate::sources::yt_search(&format!("{} {}", core(&song.title), artist)).ok()?;
    let want = core(&song.title);
    let near = |e: &&Ext| if song.dur > 0.0 && e.dur > 0.0 { (e.dur - song.dur).abs() as i64 } else { 0 };
    list.iter().filter(|e| fold(&e.title).contains(&want)).min_by_key(near).or(list.first()).cloned()
}

/// Read a playlist link and find its songs (`step` hears how far it's got).
pub fn import(api: &Api, url: &str, step: impl Fn(String)) -> Result<Imported, String> {
    let from = source(url).ok_or("Paste a playlist link from Apple Music or Spotify.")?;
    crate::api::guard()?;
    let tidal = api.has_token();
    if !tidal && crate::sources::ytdlp_path().is_none() {
        return Err("To find the songs, log in to Tidal, or press GET YT-DLP on the YouTube page.".to_string());
    }
    step(format!("Reading the {} playlist...", from));
    let read = if from == "Spotify" {
        let id = spotify_id(url).ok_or("That Spotify link has no playlist in it.")?;
        spotify_songs(&page(&format!("https://open.spotify.com/embed/playlist/{}", id))?)
    } else {
        apple_songs(&page(url.trim())?)
    };
    let (name, songs, total) = read.ok_or_else(|| format!("Couldn't read the songs on that {} page.", from))?;
    if songs.is_empty() {
        return Err(format!("No songs on that {} page (is the playlist public?).", from));
    }
    let mut out = Imported { name, from, total, ..Default::default() };
    for (i, s) in songs.iter().enumerate() {
        step(format!("Finding the songs on {}: {} of {}", if tidal { "Tidal" } else { "YouTube" }, i + 1, songs.len()));
        if tidal {
            match on_tidal(api, s) {
                Some(t) => out.tracks.push(t),
                None => out.missing.push(s.label()),
            }
        } else {
            match on_youtube(s) {
                Some(e) => {
                    out.tracks.push(e.to_track());
                    out.clips.push(e);
                }
                None => out.missing.push(s.label()),
            }
        }
    }
    crate::api::log(&format!(
        "playlist link: {} songs read from {}, {} found on {}, not found: {:?}",
        songs.len(),
        from,
        out.tracks.len(),
        if tidal { "Tidal" } else { "YouTube" },
        out.missing
    ));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(id: i64, title: &str, artist: &str, album: &str, dur: f32) -> Track {
        Track {
            id,
            title: title.into(),
            artist: artist.into(),
            artists: vec![(1, artist.into())],
            album: album.into(),
            duration: dur,
            ..Default::default()
        }
    }

    #[test]
    fn links_are_told_apart() {
        assert_eq!(source("https://music.apple.com/us/playlist/some-list/pl.u-abc"), Some("Apple Music"));
        assert_eq!(source("https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M?si=x"), Some("Spotify"));
        assert_eq!(source("https://open.spotify.com/intl-de/playlist/37i9dQZF1DXcBWIGoYBM5M"), Some("Spotify"));
        assert_eq!(source("https://music.apple.com/us/album/x/123"), None);
        assert_eq!(source("https://www.youtube.com/watch?v=x"), None);
        assert_eq!(spotify_id("https://open.spotify.com/intl-de/playlist/37i9dQZF1DX?si=abc").as_deref(), Some("37i9dQZF1DX"));
        assert_eq!(spotify_id("spotify:playlist:37i9dQ").as_deref(), Some("37i9dQ"));
    }

    #[test]
    fn apple_page_gives_its_songs_in_order() {
        let song = |t: &str, a: &str, al: &str, ms: u64| {
            format!(
                r#"{{"title":"{}","artistName":"{}","duration":{},"tertiaryLinks":[{{"title":"{}"}}],"contentDescriptor":{{"kind":"song"}}}}"#,
                t, a, ms, al
            )
        };
        let html = format!(
            r#"<meta property="og:title" content="Sunday &amp; Christmas by Someone on Apple Music"><script type="application/json" id="serialized-server-data">{{"shelves":[{{"items":[{}]}},{{"items":[{},{},{}]}}],"trackCount":40}}</script>"#,
            song("Other", "X", "Y", 1000),
            song("Magnify Him", "Rev. Timothy Wright & Michelle Prather", "The Gospel Music Celebration Pt. 2", 332613),
            song("He Reigns", "Vincent Bohanan & SOV", "A Choir Christmas", 672656),
            song("Joy to the World", "Vincent Bohanan & SOV", "A Choir Christmas", 392000),
        );
        let (name, songs, total) = apple_songs(&html).unwrap();
        assert_eq!(name, "Sunday & Christmas");
        assert_eq!(total, 40);
        assert_eq!(songs.len(), 3);
        assert_eq!(songs[0].artists, vec!["Rev. Timothy Wright", "Michelle Prather"]);
        assert_eq!(songs[2].title, "Joy to the World");
        assert!((songs[1].dur - 672.656).abs() < 0.01);
    }

    #[test]
    fn spotify_page_gives_its_songs() {
        let html = "<script id=\"__NEXT_DATA__\" type=\"application/json\">{\"props\":{\"pageProps\":{\"state\":{\"data\":{\"entity\":{\"name\":\"Hits\",\"trackList\":[{\"title\":\"BbY WOW\",\"subtitle\":\"KAROL G,\u{a0}Judeline\",\"duration\":225834}]}}}}}}</script>";
        let (name, songs, n) = spotify_songs(html).unwrap();
        assert_eq!((name.as_str(), n), ("Hits", 1));
        assert_eq!(songs[0].artists, vec!["KAROL G", "Judeline"]);
    }

    #[test]
    fn the_same_recording_wins() {
        let song = Song {
            title: "Magnify Him".into(),
            artists: vec!["Rev. Timothy Wright".into(), "Michelle Prather".into()],
            album: "The Gospel Music Celebration Pt. 2: Tribute to Bishop G.E. Patterson (Recorded Live in Memphis, Tn)".into(),
            dur: 332.6,
        };
        let ts = vec![
            track(1, "Magnify Him", "REV. TIMOTHY WRIGHT", "Sunday Morning Church Music! Vol. 2", 333.0),
            track(2, "Magnify Him", "REV. TIMOTHY WRIGHT", "The Gospel Music Celebration Pt. 2: Tribute to Bishop G.E. Patterson (Recorded Live in Memphis, Tn)", 332.0),
            track(3, "Magnify Him", "Myrna Summers", "We're Going To Make It", 381.0),
        ];
        assert_eq!(best(&song, &ts).unwrap().0.id, 2);
        // another singer's song of the same name isn't taken
        assert!(closeness(&song, &ts[2]).is_none());
    }

    #[test]
    fn titles_match_past_brackets_and_spelling() {
        let song = Song { title: "Where Jesus Is (feat. Linda McCrary-Fisher)".into(), artists: vec!["Andraé Crouch".into()], album: String::new(), dur: 248.0 };
        assert_eq!(closeness(&song, &track(1, "Where Jesus Is (feat. Linda McCrary-Fisher)", "Andrae Crouch", "The Journey", 248.0)), Some(0));
        let song = Song { title: "Lift the Saviour Up".into(), artists: vec!["O'Landa Draper And The Associates".into()], album: String::new(), dur: 202.6 };
        assert!(closeness(&song, &track(1, "Lift The Savior Up", "O'Landa Draper & The Associates", "Above and Beyond", 202.0)).is_some());
        // a live take is further than the studio one of the same length
        let song = Song { title: "Lift Him Up".into(), artists: vec!["David Walker".into()], album: String::new(), dur: 270.0 };
        let studio = closeness(&song, &track(1, "Lift Him Up", "David Walker", "So Long", 270.0)).unwrap();
        let live = closeness(&song, &track(2, "Lift Him Up (Live)", "David Walker", "Thank You", 270.0)).unwrap();
        assert!(studio < live);
        assert!(closeness(&song, &track(3, "Lift Every Voice", "David Walker", "x", 270.0)).is_none());
    }
}
