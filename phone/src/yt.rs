//! YouTube (search, a link's video, its audio, its thumbnail) through RustyPipe, YouTube's own API from Rust; what
//! the phone keeps between runs; the clipboard.

use crate::api;
use crate::decode;

/// A song in your list: a YouTube video.
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct Song {
    pub vid: String,
    pub title: String,
    pub channel: String,
    pub secs: f32,
    /// a track on the phone: its file (YouTube songs: none)
    pub path: String,
}

/// A saved loop.
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct Loop {
    pub name: String,
    pub a: f32,
    pub b: f32,
}

/// A bookmark: a spot in the song, and its name.
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct Mark {
    pub t: f32,
    pub name: String,
}

/// What you've kept for a song: its loops and bookmarks, and the speed, pitch and tempo you left it at.
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct Kept {
    pub loops: Vec<Loop>,
    pub marks: Vec<Mark>,
    /// percent (0: never set, 100)
    pub speed: u32,
    pub cents: i32,
    pub bpm: f32,
    /// your notes on the song (what to work on)
    pub notes: String,
    /// the song itself (the Loops list shows it, a phone track too) and when you last practised it (seconds since 1970)
    pub song: Song,
    pub at: u64,
}

/// Seconds since 1970.
pub(crate) fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub(crate) fn load_json<T: serde::de::DeserializeOwned + Default>(name: &str) -> T {
    std::fs::read(api::config_dir().join(name)).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

pub(crate) fn save_json<T: serde::Serialize>(name: &str, v: &T) {
    let dir = api::config_dir();
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(b) = serde_json::to_vec(v) {
        // (written beside it, then moved over it: a phone that dies mid-write keeps the old file)
        let tmp = dir.join(format!("{}.tmp", name));
        if std::fs::write(&tmp, b).is_ok() {
            let _ = std::fs::rename(&tmp, dir.join(name));
        }
    }
}

/// The video in a link: watch?v=, youtu.be/, shorts/, embed/, live/ (music.youtube.com too), or the bare 11-letter id.
pub(crate) fn video_id(link: &str) -> Option<String> {
    let l = link.trim();
    let take = |s: &str| -> Option<String> {
        let id: String = s.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect();
        (id.len() == 11).then_some(id)
    };
    if l.contains("youtu") {
        for key in ["v=", "youtu.be/", "shorts/", "embed/", "live/"] {
            if let Some(i) = l.find(key) {
                return take(&l[i + key.len()..]);
            }
        }
        return None;
    }
    // a bare id: 11 letters, digits, - and _ with at least one digit or mark (so an 11-letter word is a search)
    (l.len() == 11 && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') && l.chars().any(|c| c.is_ascii_digit() || c == '-' || c == '_'))
        .then(|| l.to_string())
}

fn client() -> Result<rustypipe::client::RustyPipe, String> {
    let dir = api::config_dir().join("yt_client");
    let _ = std::fs::create_dir_all(&dir);
    rustypipe::client::RustyPipe::builder().storage_dir(dir).build().map_err(|e| e.to_string())
}

fn runtime() -> Result<tokio::runtime::Runtime, String> {
    tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().map_err(|e| e.to_string())
}

/// A name as YouTube gives it, tidied: no combining marks (some channels underline every letter with them), no
/// doubled spaces.
pub(crate) fn tidy(s: &str) -> String {
    let t: String = s.chars().filter(|c| !matches!(*c as u32, 0x300..=0x36F | 0x1AB0..=0x1AFF | 0x20D0..=0x20FF | 0xFE20..=0xFE2F)).collect();
    t.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A channel's name as a person reads it: YouTube's own artist channels are "Name - Topic".
pub(crate) fn channel_name(s: &str) -> String {
    let t = tidy(s);
    t.strip_suffix(" - Topic").map(str::to_string).unwrap_or(t)
}

/// YouTube's videos for `q`: the quick search (YouTube's API from Rust), else yt-dlp's.
pub(crate) fn yt_search(q: &str) -> Result<Vec<Song>, String> {
    match direct_search(q) {
        Ok(v) if !v.is_empty() => Ok(v),
        first => crate::ytdlp::search(q).or_else(|e| first.and(Err(e))),
    }
}

/// A link's video: its name, channel and length (the quick way, else yt-dlp).
pub(crate) fn yt_details(vid: &str) -> Result<Song, String> {
    direct_details(vid).or_else(|e| crate::ytdlp::details(vid).map_err(|e2| format!("{} ({})", e2, e)))
}

/// A number for a text (a phone track's id on the PC copy).
pub(crate) fn hash_str(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}

/// A song's audio: a track on the phone from its file, a YouTube song as `yt_audio` gets it.
pub(crate) fn song_audio(s: &Song, keep: bool, pct: &std::sync::atomic::AtomicU32) -> Result<Vec<u8>, String> {
    if crate::device::is_file(&s.vid) {
        return crate::device::read(s);
    }
    yt_audio(&s.vid, keep, pct)
}

/// Whether a song's audio is saved on the phone.
pub(crate) fn saved(vid: &str) -> bool {
    std::fs::metadata(audio_file(vid)).map(|m| m.len() > 0).unwrap_or(false)
}

/// The saved songs: how many, and how much room they take.
pub(crate) fn saved_stats() -> (usize, u64) {
    let mut n = 0;
    let mut bytes = 0;
    if let Ok(rd) = std::fs::read_dir(crate::api::config_dir().join("yt")) {
        for e in rd.flatten() {
            if e.path().extension().is_some_and(|x| x == "m4a") {
                n += 1;
                bytes += e.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
    }
    (n, bytes)
}

/// Every saved song deleted.
pub(crate) fn clear_saved() {
    let _ = std::fs::remove_dir_all(crate::api::config_dir().join("yt"));
}

/// A size as people say it: 7.9 MB, 1.2 GB.
pub(crate) fn fmt_size(b: u64) -> String {
    let mb = b as f64 / 1_048_576.0;
    if mb >= 1024.0 {
        format!("{:.1} GB", mb / 1024.0)
    } else {
        format!("{:.1} MB", mb)
    }
}

/// A video's audio (AAC in an m4a, which the player reads): the saved copy, else fetched by yt-dlp (the old direct
/// way when yt-dlp can't). `keep`: saved for next time (SAVING on); else gone once it's read. `pct` follows the
/// download.
pub(crate) fn yt_audio(vid: &str, keep: bool, pct: &std::sync::atomic::AtomicU32) -> Result<Vec<u8>, String> {
    let file = audio_file(vid);
    if let Ok(b) = std::fs::read(&file) {
        if !b.is_empty() {
            return Ok(b);
        }
    }
    let got = match crate::ytdlp::audio(vid, &file, pct) {
        Ok(()) => std::fs::read(&file).map_err(|e| e.to_string()),
        Err(e) => {
            crate::api::log(&format!("yt-dlp couldn't get {}: {}", vid, e));
            // (yt-dlp out of date is the usual reason: brought up to date, then once more)
            let again = crate::ytdlp::update_due() && crate::ytdlp::update().is_ok() && crate::ytdlp::audio(vid, &file, pct).is_ok();
            if again {
                std::fs::read(&file).map_err(|e| e.to_string())
            } else {
                direct_audio(vid).map_err(|_| e)
            }
        }
    };
    if !keep {
        let _ = std::fs::remove_file(&file);
    }
    got
}

/// A song's name and who made it, for showing: YouTube's clutter off the title ("(Official Video)", "(4K
/// Remaster)"...), and "Artist - Song" split into the two (else the channel is who).
pub(crate) fn parts(s: &Song) -> (String, String) {
    let mut t = s.title.clone();
    // bracketed bits that only describe the upload
    const CLUTTER: [&str; 14] = ["official", "video", "audio", "lyric", "visuali", "remaster", "4k", " hd", "(hd", "[hd", " hq", "(hq", "m/v", "full album"];
    loop {
        let open: Vec<usize> = t.char_indices().filter(|(_, c)| *c == '(' || *c == '[').map(|(i, _)| i).collect();
        let mut cut = None;
        for i in open.into_iter().rev() {
            let close = if t[i..].starts_with('(') { ')' } else { ']' };
            if let Some(j) = t[i..].find(close) {
                let inner = format!(" {}", t[i..i + j + 1].to_lowercase());
                if CLUTTER.iter().any(|c| inner.contains(c)) {
                    cut = Some((i, i + j + close.len_utf8()));
                    break;
                }
            }
        }
        match cut {
            Some((a, b)) => t = format!("{}{}", &t[..a], &t[b..]),
            None => break,
        }
    }
    let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
    let t = t.trim_end_matches(|c: char| c == '-' || c == '|' || c.is_whitespace()).to_string();
    // (a track on the phone has its own artist and title: the title stays whole)
    if crate::device::is_file(&s.vid) && !s.channel.is_empty() {
        return (s.channel.clone(), if t.is_empty() { s.title.clone() } else { t });
    }
    for sep in [" - ", " \u{2013} ", " \u{2014} "] {
        if let Some((a, b)) = t.split_once(sep) {
            if !a.trim().is_empty() && !b.trim().is_empty() && a.chars().count() <= 40 {
                return (a.trim().to_string(), b.trim().to_string());
            }
        }
    }
    (s.channel.clone(), if t.is_empty() { s.title.clone() } else { t })
}

/// YouTube's videos for `q` straight from its API (RustyPipe) (no Shorts, no live streams).
fn direct_search(q: &str) -> Result<Vec<Song>, String> {
    let rt = runtime()?;
    rt.block_on(async {
        let rp = client()?;
        let r = rp.query().search::<rustypipe::model::VideoItem, _>(q).await.map_err(|e| e.to_string())?;
        Ok(r.items
            .items
            .into_iter()
            .filter(|v| !v.is_short && !v.is_live && !v.is_upcoming)
            .map(|v| Song { vid: v.id, title: tidy(&v.name), channel: channel_name(&v.channel.map(|c| c.name).unwrap_or_default()), secs: v.duration.unwrap_or(0) as f32, ..Default::default() })
            .collect())
    })
}

/// A link's video: its name, channel and length.
fn direct_details(vid: &str) -> Result<Song, String> {
    let rt = runtime()?;
    rt.block_on(async {
        let rp = client()?;
        let p = rp.query().player(vid).await.map_err(|e| e.to_string())?;
        if p.details.is_live {
            return Err("it's a live stream".to_string());
        }
        Ok(Song {
            vid: vid.to_string(),
            title: tidy(&p.details.name.unwrap_or_else(|| vid.to_string())),
            channel: channel_name(&p.details.channel_name.unwrap_or_default()),
            secs: p.details.duration as f32,
            ..Default::default()
        })
    })
}

/// Where a video's audio is kept on the phone.
pub(crate) fn audio_file(vid: &str) -> std::path::PathBuf {
    api::config_dir().join("yt").join(format!("{}.m4a", vid))
}

/// A video's audio straight from YouTube's API (RustyPipe): only some videos come whole this way now.
fn direct_audio(vid: &str) -> Result<Vec<u8>, String> {
    let file = audio_file(vid);
    let rt = runtime()?;
    let (url, size, ua) = rt.block_on(async {
        let rp = client()?;
        let p = rp.query().player(vid).await.map_err(|e| e.to_string())?;
        // (the downloads go as the same app the stream was asked for as)
        let ua = rp.query().user_agent(p.client_type).to_string();
        p.audio_streams
            .iter()
            .filter(|s| s.format == rustypipe::model::AudioFormat::M4a)
            .max_by_key(|s| s.bitrate)
            .map(|s| (s.url.clone(), s.size, ua))
            .ok_or_else(|| "no audio this phone can play".to_string())
    })?;
    // in pieces: YouTube slows down one long download
    let http = reqwest::blocking::Client::builder().user_agent(ua).timeout(std::time::Duration::from_secs(60)).build().map_err(|e| e.to_string())?;
    let mut out: Vec<u8> = Vec::with_capacity(size as usize);
    let piece = 1024 * 1024u64;
    let mut at = 0u64;
    while at < size.max(1) {
        let end = (at + piece - 1).min(size.saturating_sub(1));
        let b = http
            .get(&url)
            .header("Range", format!("bytes={}-{}", at, end))
            .send()
            .and_then(|r| r.error_for_status())
            .and_then(|r| r.bytes())
            .map_err(|e| {
                // (YouTube lets some videos stop after the first part: say so plainly)
                if e.status().is_some_and(|s| s.as_u16() == 403) && at > 0 {
                    "YouTube stopped the download partway (this song can't be fetched yet)".to_string()
                } else if e.status().is_some_and(|s| s.as_u16() == 403) {
                    "YouTube refused the download".to_string()
                } else {
                    e.to_string()
                }
            })?;
        if b.is_empty() {
            break;
        }
        at += b.len() as u64;
        out.extend_from_slice(&b);
        if size == 0 {
            break;
        }
    }
    if out.is_empty() {
        return Err("the download came back empty".to_string());
    }
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(&file, &out);
    Ok(out)
}

/// A video's thumbnail, the middle square of it (as the desktop's covers are square).
pub(crate) fn yt_thumb(vid: &str) -> Option<eframe::egui::ColorImage> {
    let dir = api::config_dir().join("thumbs");
    let file = dir.join(format!("{}.jpg", vid));
    let bytes = match std::fs::read(&file) {
        Ok(b) => b,
        Err(_) => {
            let url = format!("https://i.ytimg.com/vi/{}/mqdefault.jpg", vid);
            let b = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(20))
                .build()
                .ok()?
                .get(&url)
                .send()
                .ok()?
                .error_for_status()
                .ok()?
                .bytes()
                .ok()?
                .to_vec();
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(&file, &b);
            b
        }
    };
    let img = image::load_from_memory(&bytes).ok()?.to_rgba8();
    let (w, h) = (img.width(), img.height());
    let s = w.min(h);
    let sq = image::imageops::crop_imm(&img, (w - s) / 2, (h - s) / 2, s, s).to_image();
    let sq = image::imageops::resize(&sq, 160, 160, image::imageops::FilterType::Triangle);
    Some(eframe::egui::ColorImage::from_rgba_unmultiplied([160, 160], sq.as_raw()))
}

/// A song's length and a 400-peak waveform (0-255), from its encoded bytes.
pub(crate) fn wave(bytes: &[u8]) -> (f32, Vec<u8>) {
    use rodio::Source;
    let Ok(src) = decode::SymSource::new(bytes.to_vec()) else { return (0.0, Vec::new()) };
    let (ch, rate) = (src.channels().max(1) as usize, src.sample_rate().max(1) as f32);
    let n = 800;
    let mut peaks_raw: Vec<u16> = Vec::new();
    let mut cur: u16 = 0;
    let mut count = 0usize;
    // (one pass, keeping a running peak per 1/20 s: no copy of the whole song's samples)
    let per = (rate as usize / 20).max(1) * ch;
    let mut total = 0u64;
    for v in src {
        cur = cur.max(v.unsigned_abs());
        count += 1;
        total += 1;
        if count == per {
            peaks_raw.push(cur);
            cur = 0;
            count = 0;
        }
    }
    if count > 0 {
        peaks_raw.push(cur);
    }
    let dur = total as f32 / ch as f32 / rate;
    if peaks_raw.is_empty() {
        return (0.0, Vec::new());
    }
    let m = peaks_raw.len();
    let peaks = (0..n)
        .map(|k| {
            let s = k * m / n;
            let e = ((k + 1) * m / n).max(s + 1).min(m);
            let v = peaks_raw[s.min(m - 1)..e].iter().copied().max().unwrap_or(0);
            (v as u32 * 255 / 32768).min(255) as u8
        })
        .collect();
    (dur.max(0.0), peaks)
}

/// What is on the phone's clipboard, through Android's ClipboardManager.
pub(crate) fn clipboard() -> Option<String> {
    #[cfg(target_os = "android")]
    {
        use jni::objects::{JObject, JString};
        let ctx = ndk_context::android_context();
        let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }.ok()?;
        let mut env = vm.attach_current_thread().ok()?;
        let activity = unsafe { JObject::from_raw(ctx.context().cast()) };
        let name = env.new_string("clipboard").ok()?;
        let cm = env.call_method(&activity, "getSystemService", "(Ljava/lang/String;)Ljava/lang/Object;", &[(&name).into()]).ok()?.l().ok()?;
        let clip = env.call_method(&cm, "getPrimaryClip", "()Landroid/content/ClipData;", &[]).ok()?.l().ok()?;
        if clip.is_null() {
            return None;
        }
        let item = env.call_method(&clip, "getItemAt", "(I)Landroid/content/ClipData$Item;", &[0i32.into()]).ok()?.l().ok()?;
        let text = env.call_method(&item, "getText", "()Ljava/lang/CharSequence;", &[]).ok()?.l().ok()?;
        if text.is_null() {
            return None;
        }
        let s = env.call_method(&text, "toString", "()Ljava/lang/String;", &[]).ok()?.l().ok()?;
        let s: String = env.get_string(&JString::from(s)).ok()?.into();
        return Some(s);
    }
    #[allow(unreachable_code)]
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn links() {
        assert_eq!(video_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=1").as_deref(), Some("dQw4w9WgXcQ"));
        assert_eq!(video_id("https://youtu.be/dQw4w9WgXcQ?si=abc").as_deref(), Some("dQw4w9WgXcQ"));
        assert_eq!(video_id("https://music.youtube.com/watch?v=dQw4w9WgXcQ&list=x").as_deref(), Some("dQw4w9WgXcQ"));
        assert_eq!(video_id("https://youtube.com/shorts/dQw4w9WgXcQ").as_deref(), Some("dQw4w9WgXcQ"));
        assert_eq!(video_id("dQw4w9WgXcQ").as_deref(), Some("dQw4w9WgXcQ"));
        assert_eq!(video_id("wayne shorter juju"), None);
        assert_eq!(video_id("Coltranesss"), None);
        assert_eq!(tidy("Wayne S\u{332}h\u{332}orter  \u{2013} Juju"), "Wayne Shorter \u{2013} Juju");
    }
}
