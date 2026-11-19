//! Audio that doesn't come from Tidal: files on this computer, YouTube clips (through yt-dlp),
//! plus helpers that work on any audio: waveform peaks and WAV export of a loop.

use crate::decode::SymSource;
use crate::store::{hash_id, Ext};
use rodio::Source;
use std::path::{Path, PathBuf};
use std::io::{BufRead, Read};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use symphonia::core::codecs::CODEC_TYPE_NULL;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub const AUDIO_EXT: [&str; 9] = ["mp3", "flac", "wav", "m4a", "aac", "mp4", "ogg", "oga", "alac"];

/// Where a non-Tidal track comes from.
#[derive(Clone)]
pub enum Src {
    File(PathBuf),
    Yt(String),
}

pub fn is_audio(p: &Path) -> bool {
    p.extension().and_then(|e| e.to_str()).map(|e| AUDIO_EXT.contains(&e.to_ascii_lowercase().as_str())).unwrap_or(false)
}

/// All audio files below `dir` (a few levels deep, capped so a huge drive can't hang the app).
pub fn scan_folder(dir: &Path, depth: u32, out: &mut Vec<PathBuf>) {
    if depth > 6 || out.len() > 5000 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            scan_folder(&p, depth + 1, out);
        } else if is_audio(&p) {
            out.push(p);
        }
    }
}

/// Length in seconds, read from the file header (or by walking the packets when there is none).
pub fn probe_duration(path: &Path) -> f32 {
    let Ok(f) = std::fs::File::open(path) else { return 0.0 };
    let mss = MediaSourceStream::new(Box::new(f), Default::default());
    let mut hint = Hint::new();
    if let Some(e) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(e);
    }
    let Ok(probed) = symphonia::default::get_probe().format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
    else {
        return 0.0;
    };
    let mut format = probed.format;
    let (id, params) = {
        let Some(t) = format.tracks().iter().find(|t| t.codec_params.codec != CODEC_TYPE_NULL) else {
            return 0.0;
        };
        (t.id, t.codec_params.clone())
    };
    let rate = params.sample_rate.unwrap_or(44100) as f64;
    if let Some(n) = params.n_frames {
        if let Some(tb) = params.time_base {
            let t = tb.calc_time(n);
            return (t.seconds as f64 + t.frac) as f32;
        }
        return (n as f64 / rate) as f32;
    }
    let mut frames: u64 = 0;
    while let Ok(pkt) = format.next_packet() {
        if pkt.track_id() == id {
            frames += pkt.dur;
        }
    }
    (frames as f64 / rate) as f32
}

pub fn ext_for_file(path: &Path) -> Ext {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("?").to_string();
    let parent = path.parent().and_then(|p| p.file_name()).and_then(|s| s.to_str()).unwrap_or("").to_string();
    let (mut artist, mut title) = match stem.split_once(" - ") {
        Some((a, t)) => (a.trim().to_string(), t.trim().to_string()),
        None => (parent, stem.clone()),
    };
    // the file's own tags, when it has them, name it better than its file name
    let mut album = String::new();
    if let Some((a, t, al)) = tag_names(path) {
        if !t.is_empty() {
            title = t;
        }
        if !a.is_empty() {
            artist = a;
        }
        album = al;
    }
    let src = path.to_string_lossy().to_string();
    Ext {
        kind: "file".to_string(),
        id: hash_id(&format!("file:{}", src)),
        dur: probe_duration(path),
        src,
        title,
        artist,
        album,
        cover: String::new(),
    }
}

/// The artist, title and album in a file's tags.
fn tag_names(path: &Path) -> Option<(String, String, String)> {
    use lofty::prelude::*;
    let tagged = lofty::read_from_path(path).ok()?;
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag())?;
    Some((
        tag.artist().map(|s| s.to_string()).unwrap_or_default(),
        tag.title().map(|s| s.to_string()).unwrap_or_default(),
        tag.album().map(|s| s.to_string()).unwrap_or_default(),
    ))
}

/// A file's cover picture: the one inside it (its tags), else a cover image in its folder
/// (cover / folder / front / album, .jpg or .png).
pub fn file_art(path: &Path) -> Option<Vec<u8>> {
    use lofty::prelude::*;
    if let Ok(tagged) = lofty::read_from_path(path) {
        for tag in tagged.tags() {
            if let Some(p) = tag.pictures().first() {
                return Some(p.data().to_vec());
            }
        }
    }
    let dir = path.parent()?;
    for name in ["cover", "folder", "front", "album", "Cover", "Folder", "Front", "AlbumArt"] {
        for ext in ["jpg", "jpeg", "png"] {
            if let Ok(b) = std::fs::read(dir.join(format!("{}.{}", name, ext))) {
                return Some(b);
            }
        }
    }
    None
}

/// Show a file in the system's file browser, selected.
pub fn reveal(path: &str) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("explorer").raw_arg(explorer_select(path)).spawn().map(|_| ())
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg("-R").arg(path).spawn().map(|_| ())
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        let d = Path::new(path).parent().unwrap_or(Path::new(path));
        std::process::Command::new("xdg-open").arg(d).spawn().map(|_| ())
    }
}

/// What Explorer is given to open a file's folder with the file picked: /select,"D:\Music\a song.mp3" -
/// backslashes, no long-path prefix, the path alone in quotes (the whole thing quoted, Explorer ignores it and opens
/// Documents).
pub fn explorer_select(path: &str) -> String {
    let p = path.replace('/', "\\");
    let p = p.strip_prefix(r"\\?\").unwrap_or(&p);
    format!("/select,\"{}\"", p)
}

// ------------------------------------------------------------------ YouTube
fn exe_name() -> &'static str {
    if cfg!(windows) {
        "yt-dlp.exe"
    } else {
        "yt-dlp"
    }
}

pub fn ytdlp_dir() -> PathBuf {
    crate::api::config_dir().join("bin")
}

fn command(p: &Path) -> Command {
    let mut c = Command::new(p);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000); // no console window
    }
    c.stdin(Stdio::null());
    c
}

/// yt-dlp next to the app, in the Backline folder, or anywhere on PATH. (On the Mac, its folder build first; a Mac
/// that only has the one-file build gets the folder build in the background: `mac_folder_soon`.)
pub fn ytdlp_path() -> Option<PathBuf> {
    // (asked every frame on the YouTube and SoundCloud pages: looked for at most every 10 seconds, as it may start
    // yt-dlp itself to see if it's on PATH)
    let mut seen = YTDLP_SEEN.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((at, p)) = seen.as_ref() {
        if at.elapsed() < std::time::Duration::from_secs(10) {
            return p.clone();
        }
    }
    let p = find_ytdlp();
    *seen = Some((std::time::Instant::now(), p.clone()));
    p
}

/// Forget where yt-dlp was found (it was just downloaded): looked for again next time.
pub fn ytdlp_changed() {
    *YTDLP_SEEN.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

/// Where yt-dlp was found last, and when.
static YTDLP_SEEN: std::sync::Mutex<Option<(std::time::Instant, Option<PathBuf>)>> = std::sync::Mutex::new(None);

fn find_ytdlp() -> Option<PathBuf> {
    let mut cands = vec![ytdlp_dir().join(exe_name())];
    if cfg!(target_os = "macos") {
        cands.insert(0, ytdlp_dir().join("yt-dlp_macos").join("yt-dlp_macos"));
    }
    if let Ok(me) = std::env::current_exe() {
        if let Some(d) = me.parent() {
            cands.push(d.join(exe_name()));
        }
    }
    for c in cands {
        if c.is_file() {
            #[cfg(target_os = "macos")]
            mac_folder_soon(&c);
            return Some(c);
        }
    }
    let p = PathBuf::from(exe_name());
    if command(&p).arg("--version").output().map(|o| o.status.success()).unwrap_or(false) {
        return Some(p);
    }
    None
}

/// yt-dlp's one-file build unpacks itself every time it runs, and a Mac checks what was unpacked each time: many
/// seconds before every song. A Mac that still has it (from before Backline used the folder build) gets the folder
/// build in the background, once a run; the one-file build is used till then.
#[cfg(target_os = "macos")]
fn mac_folder_soon(found: &Path) {
    use std::sync::atomic::{AtomicBool, Ordering};
    static ASKED: AtomicBool = AtomicBool::new(false);
    if found != ytdlp_dir().join(exe_name()) || ASKED.swap(true, Ordering::Relaxed) || crate::api::guard().is_err() {
        return;
    }
    std::thread::spawn(|| {
        let client = reqwest::blocking::Client::builder().timeout(Duration::from_secs(300)).build();
        match client.ok().and_then(|c| ytdlp_mac_folder(&c)) {
            Some(p) => crate::api::log(&format!("youtube: yt-dlp's faster Mac build is in place ({})", p.display())),
            None => crate::api::log("youtube: couldn't get yt-dlp's faster Mac build; the one-file build stays"),
        }
    });
}

/// The Mac's folder build of yt-dlp under bin/yt-dlp_macos/: unpacked beside it first and checked to run, then put
/// in place (so one half unpacked is never used), and run once more there (the Mac checks it the first time). None:
/// use the one-file build instead.
#[cfg(target_os = "macos")]
fn ytdlp_mac_folder(client: &reqwest::blocking::Client) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let url = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos.zip";
    let bytes = client.get(url).send().ok()?.error_for_status().ok()?.bytes().ok()?;
    let fin = ytdlp_dir().join("yt-dlp_macos");
    let dir = ytdlp_dir().join("yt-dlp_macos.new");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).ok()?;
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes.to_vec())).ok()?;
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).ok()?;
        let Some(rel) = f.enclosed_name() else { continue };
        let out = dir.join(rel);
        if f.is_dir() {
            let _ = std::fs::create_dir_all(&out);
            continue;
        }
        if let Some(p) = out.parent() {
            let _ = std::fs::create_dir_all(p);
        }
        let mut data = Vec::new();
        std::io::Read::read_to_end(&mut f, &mut data).ok()?;
        // a link inside the folder (the zip keeps it as its target's name)
        if f.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000) {
            let target = String::from_utf8_lossy(&data).to_string();
            let _ = std::os::unix::fs::symlink(target, &out);
            continue;
        }
        std::fs::write(&out, &data).ok()?;
        let _ = std::fs::set_permissions(&out, std::fs::Permissions::from_mode(f.unix_mode().unwrap_or(0o755) | 0o500));
    }
    // the zip may hold the folder itself
    let exe_in = |d: &Path| [d.join("yt-dlp_macos"), d.join("yt-dlp_macos").join("yt-dlp_macos")].into_iter().find(|p| p.is_file());
    let runs = |p: &Path| command(p).arg("--version").output().map(|o| o.status.success()).unwrap_or(false);
    if !exe_in(&dir).is_some_and(|e| runs(&e)) {
        let _ = std::fs::remove_dir_all(&dir);
        return None;
    }
    let _ = std::fs::remove_dir_all(&fin);
    std::fs::rename(&dir, &fin).ok()?;
    let exe = exe_in(&fin)?;
    runs(&exe).then_some(exe)
}

/// Download (or update) yt-dlp from its official GitHub release into the Backline folder.
pub fn ytdlp_download() -> Result<PathBuf, String> {
    #[cfg(target_os = "macos")]
    {
        let client = reqwest::blocking::Client::builder().timeout(Duration::from_secs(300)).build().map_err(|e| e.to_string())?;
        if let Some(p) = ytdlp_mac_folder(&client) {
            return Ok(p);
        }
    }
    let url = if cfg!(windows) {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"
    } else if cfg!(target_os = "macos") {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos"
    } else {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp"
    };
    let client = reqwest::blocking::Client::builder().timeout(Duration::from_secs(300)).build().map_err(|e| e.to_string())?;
    let bytes = client
        .get(url)
        .send()
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .bytes()
        .map_err(|e| e.to_string())?;
    if bytes.len() < 1_000_000 {
        return Err("download looks incomplete".to_string());
    }
    let dir = ytdlp_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let tmp = dir.join("yt-dlp.download");
    std::fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
    let fin = dir.join(exe_name());
    std::fs::rename(&tmp, &fin).map_err(|e| format!("could not replace yt-dlp ({})", e))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&fin, std::fs::Permissions::from_mode(0o755));
    }
    Ok(fin)
}

fn valid_id(s: &str) -> bool {
    s.len() == 11 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// The 11-character video id out of any common YouTube link (or a bare id).
pub fn yt_video_id(input: &str) -> Option<String> {
    let t = input.trim();
    if valid_id(t) {
        return Some(t.to_string());
    }
    for marker in ["v=", "youtu.be/", "/shorts/", "/embed/", "/live/"] {
        if let Some(i) = t.find(marker) {
            let rest = &t[i + marker.len()..];
            let id: String = rest.chars().take(11).collect();
            if valid_id(&id) {
                return Some(id);
            }
        }
    }
    None
}

/// A yt-dlp run to the end, with how long it took in the log (on an older Mac, starting yt-dlp is most of it).
trait Timed {
    fn timed(&mut self, what: &str) -> std::io::Result<std::process::Output>;
}

impl Timed for Command {
    fn timed(&mut self, what: &str) -> std::io::Result<std::process::Output> {
        let t0 = Instant::now();
        let r = self.output();
        let how = match &r {
            Ok(o) if !o.status.success() => format!(", failed: {}", last_line(&o.stderr)),
            Ok(_) => String::new(),
            Err(e) => format!(", couldn't run: {}", e),
        };
        crate::api::log(&format!("yt-dlp {}: {:.1} s{}", what, t0.elapsed().as_secs_f32(), how));
        r
    }
}

/// Which yt-dlp it is, for the log: the Mac's folder build, the one in Backline's folder (on a Mac, the one-file
/// build), or where it is.
fn build_name(exe: &Path) -> String {
    if exe.parent().is_some_and(|d| d.ends_with("yt-dlp_macos")) {
        "the folder build".to_string()
    } else if exe == ytdlp_dir().join(exe_name()) {
        if cfg!(target_os = "macos") { "the one-file build".to_string() } else { format!("{} in Backline's folder", exe_name()) }
    } else {
        exe.display().to_string()
    }
}

fn last_line(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("unknown error").trim().to_string()
}

/// Which browser's SoundCloud sign-in yt-dlp borrows: 0 none, 1 firefox, 2 chrome, 3 edge.
pub static SC_BROWSER: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
pub const SC_BROWSERS: [&str; 4] = ["OFF", "FIREFOX", "CHROME", "EDGE"];

/// `--cookies-from-browser <name>` for SoundCloud requests when switched on.
fn cookie_args() -> Vec<String> {
    match SC_BROWSER.load(std::sync::atomic::Ordering::Relaxed) {
        1 => vec!["--cookies-from-browser".into(), "firefox".into()],
        2 => vec!["--cookies-from-browser".into(), "chrome".into()],
        3 => vec!["--cookies-from-browser".into(), "edge".into()],
        _ => Vec::new(),
    }
}

/// A YouTube video id, or (SoundCloud and others) a full link.
fn media_url(src: &str) -> String {
    if src.starts_with("http") {
        src.to_string()
    } else {
        format!("https://www.youtube.com/watch?v={}", src)
    }
}

/// A file-name-safe stem for the temporary download of `src`.
fn file_stem(src: &str) -> String {
    if src.starts_with("http") {
        format!("sc{}", hash_id(src).unsigned_abs())
    } else {
        src.to_string()
    }
}

/// "soundcloud.com/some-artist/cool-track" -> ("Some Artist", "Cool Track"), when the link has that shape.
fn slug_meta(url: &str) -> Option<(String, String)> {
    let rest = url.split("://").nth(1)?;
    let (host, path) = rest.split_once('/')?;
    if !host.ends_with("soundcloud.com") || host.starts_with("api.") {
        return None;
    }
    let segs: Vec<&str> = path.split(['/', '?']).filter(|s| !s.is_empty()).collect();
    if segs.len() < 2 || ["sets", "likes", "reposts", "tracks", "albums", "popular-tracks"].contains(&segs[1]) {
        return None;
    }
    let pretty = |s: &str| -> String {
        s.split('-')
            .map(|w| {
                let mut c = w.chars();
                c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
            })
            .collect::<Vec<_>>()
            .join(" ")
    };
    Some((pretty(segs[0]), pretty(segs[1])))
}

/// SoundCloud entries as yt-dlp lists them (one JSON object per line, "flat" = no per-track requests).
fn parse_entries(bytes: &[u8]) -> Vec<Ext> {
    let mut out: Vec<Ext> = Vec::new();
    for line in String::from_utf8_lossy(bytes).lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else { continue };
        let url = v["webpage_url"].as_str().or_else(|| v["url"].as_str()).unwrap_or("");
        if !url.starts_with("http") || !url.contains("soundcloud.com") || out.iter().any(|e| e.src == url) {
            continue;
        }
        let cover = v["thumbnail"]
            .as_str()
            .map(|s| s.to_string())
            .or_else(|| v["thumbnails"].as_array().and_then(|a| a.last()).and_then(|t| t["url"].as_str()).map(|s| s.to_string()))
            .unwrap_or_default();
        out.push(Ext {
            kind: "yt".to_string(),
            id: hash_id(&format!("yt:{}", url)),
            src: url.to_string(),
            // the quick listing has no titles: borrow them from the link until the real ones arrive
            title: v["title"]
                .as_str()
                .map(|s| s.to_string())
                .or_else(|| slug_meta(url).map(|m| m.1))
                .unwrap_or_else(|| "SoundCloud track".to_string()),
            artist: v["uploader"]
                .as_str()
                .or_else(|| v["channel"].as_str())
                .map(|s| s.to_string())
                .or_else(|| slug_meta(url).map(|m| m.0))
                .unwrap_or_else(|| "SoundCloud".to_string()),
            album: String::new(),
            dur: v["duration"].as_f64().unwrap_or(0.0) as f32,
            cover,
        });
    }
    out
}

/// Search SoundCloud (no account needed).
pub fn sc_search(query: &str) -> Result<Vec<Ext>, String> {
    let exe = ytdlp_path().ok_or_else(|| "yt-dlp is not installed - press GET YT-DLP".to_string())?;
    let out = command(&exe)
        .args(["--no-warnings", "--flat-playlist", "--dump-json"])
        .args(cookie_args())
        .arg(format!("scsearch15:{}", query))
        .timed("SoundCloud search")
        .map_err(|e| format!("could not run yt-dlp: {}", e))?;
    let list = parse_entries(&out.stdout);
    if list.is_empty() {
        return Err(if out.status.success() { "nothing found".to_string() } else { last_line(&out.stderr) });
    }
    Ok(list)
}

/// Search YouTube (no account needed): the first dozen videos.
pub fn yt_search(query: &str) -> Result<Vec<Ext>, String> {
    let exe = ytdlp_path().ok_or_else(|| "yt-dlp is not installed - press GET YT-DLP".to_string())?;
    let out = command(&exe)
        .args(["--no-warnings", "--flat-playlist", "--dump-json"])
        .arg(format!("ytsearch12:{}", query))
        .timed("YouTube search")
        .map_err(|e| format!("could not run yt-dlp: {}", e))?;
    let mut list: Vec<Ext> = Vec::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else { continue };
        let Some(vid) = v["id"].as_str().filter(|s| valid_id(s)) else { continue };
        if list.iter().any(|e| e.src == vid) {
            continue;
        }
        list.push(Ext {
            kind: "yt".to_string(),
            id: hash_id(&format!("yt:{}", vid)),
            src: vid.to_string(),
            title: v["title"].as_str().unwrap_or("YouTube clip").to_string(),
            artist: v["channel"].as_str().or_else(|| v["uploader"].as_str()).unwrap_or("YouTube").replace(" - Topic", ""),
            album: String::new(),
            dur: v["duration"].as_f64().unwrap_or(0.0) as f32,
            cover: format!("https://i.ytimg.com/vi/{}/hqdefault.jpg", vid),
        });
    }
    if list.is_empty() {
        return Err(if out.status.success() { "nothing found".to_string() } else { last_line(&out.stderr) });
    }
    Ok(list)
}

/// A SoundCloud track, playlist, likes page or profile link: list what is in it (first 100).
pub fn sc_list(url: &str) -> Result<Vec<Ext>, String> {
    let exe = ytdlp_path().ok_or_else(|| "yt-dlp is not installed - press GET YT-DLP".to_string())?;
    let out = command(&exe)
        .args(["--no-warnings", "--flat-playlist", "--dump-json", "--playlist-end", "100"])
        .args(cookie_args())
        .arg(url.trim())
        .timed("SoundCloud list")
        .map_err(|e| format!("could not run yt-dlp: {}", e))?;
    let mut list = parse_entries(&out.stdout);
    if list.is_empty() && out.status.success() {
        // a single track comes back as one full object
        list = parse_entries(&out.stdout);
    }
    if list.is_empty() {
        return Err(if out.status.success() { "nothing found at that link".to_string() } else { last_line(&out.stderr) });
    }
    Ok(list)
}

/// A YouTube playlist link (not a single video that happens to sit in one).
/// A playlist file (.m3u, .m3u8, .pls)?
pub fn is_playlist_file(p: &Path) -> bool {
    p.extension().and_then(|e| e.to_str()).is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "m3u" | "m3u8" | "pls"))
}

/// The songs a playlist file lists, in order (paths beside it work as well as full ones), and its name.
pub fn read_playlist_file(p: &Path) -> Option<(String, Vec<PathBuf>)> {
    let bytes = std::fs::read(p).ok()?;
    let text = String::from_utf8_lossy(&bytes);
    let dir = p.parent().map(Path::to_path_buf).unwrap_or_default();
    let pls = p.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("pls"));
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim().trim_start_matches('\u{feff}');
        let entry = if pls {
            match line.split_once('=') {
                Some((k, v)) if k.to_ascii_lowercase().starts_with("file") => v.trim(),
                _ => continue,
            }
        } else if line.is_empty() || line.starts_with('#') {
            continue;
        } else {
            line
        };
        let entry = entry.strip_prefix("file:///").or_else(|| entry.strip_prefix("file://")).unwrap_or(entry);
        let entry = entry.replace("%20", " ");
        let path = PathBuf::from(&entry);
        let path = if path.is_absolute() { path } else { dir.join(path) };
        if path.exists() && is_audio(&path) {
            out.push(path);
        }
    }
    let name = p.file_stem().and_then(|s| s.to_str()).unwrap_or("Playlist").to_string();
    Some((name, out))
}

/// A YouTube link to a playlist: a playlist page, or a video played from a playlist (watch?v=...&list=...). A video
/// played from a Mix (list=RD..., made up by YouTube around that video) stays that one video.
pub fn is_yt_list(s: &str) -> bool {
    let s = s.trim();
    match yt_list_id(s) {
        Some(id) => s.contains("youtu") && (s.contains("playlist?list=") || !id.starts_with("RD")),
        None => false,
    }
}

/// The playlist a YouTube link points to (its list=...).
pub fn yt_list_id(s: &str) -> Option<String> {
    let i = s.find("list=")?;
    let prev = s[..i].chars().last();
    if !matches!(prev, Some('?') | Some('&')) {
        return None;
    }
    let id: String = s[i + 5..].chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect();
    (!id.is_empty()).then_some(id)
}

/// A playlist link as a plain playlist page (so a video link with a list= reads the whole playlist).
pub fn yt_list_url(s: &str) -> String {
    match yt_list_id(s) {
        Some(id) => format!("https://www.youtube.com/playlist?list={}", id),
        None => s.trim().to_string(),
    }
}

/// A readable name from a link's last part: ".../sets/my-cool-mix" -> "My Cool Mix".
pub fn list_name(url: &str) -> String {
    let last = url.split('?').next().unwrap_or(url).trim_end_matches('/').rsplit('/').next().unwrap_or("playlist");
    let name: Vec<String> = last
        .split(['-', '_'])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
        })
        .collect();
    if name.is_empty() {
        "Playlist".to_string()
    } else {
        name.join(" ")
    }
}

/// How many videos of the playlist being read have come in so far (shown while it reads).
pub static YT_READ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// A YouTube playlist: its name and everything in it, a quick listing without per-video requests. Read as it comes
/// (the count shows), at most 1000 - and a YouTube Mix (list=RD...), which never ends, its first 50.
pub fn yt_list(url: &str) -> Result<(String, Vec<Ext>), String> {
    use std::io::BufRead;
    let exe = ytdlp_path().ok_or_else(|| "yt-dlp is not installed - press GET YT-DLP".to_string())?;
    let cap = if url.contains("list=RD") { "50" } else { "1000" };
    let mut child = command(&exe)
        .args(["--no-warnings", "--flat-playlist", "--dump-json", "--playlist-end", cap])
        .arg(url.trim())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("could not run yt-dlp: {}", e))?;
    YT_READ.store(0, std::sync::atomic::Ordering::Relaxed);
    let mut lines_in: Vec<String> = Vec::new();
    if let Some(so) = child.stdout.take() {
        for line in std::io::BufReader::new(so).lines().map_while(Result::ok) {
            lines_in.push(line);
            YT_READ.store(lines_in.len(), std::sync::atomic::Ordering::Relaxed);
        }
    }
    let out = child.wait_with_output().map_err(|e| format!("yt-dlp: {}", e))?;
    let mut list: Vec<Ext> = Vec::new();
    let mut name = String::new();
    let mut seen = std::collections::HashSet::new();
    for line in lines_in.iter() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else { continue };
        if name.is_empty() {
            name = v["playlist_title"].as_str().or_else(|| v["playlist"].as_str()).unwrap_or("").to_string();
        }
        let Some(vid) = v["id"].as_str().filter(|s| valid_id(s)) else { continue };
        if !seen.insert(vid.to_string()) {
            continue;
        }
        list.push(Ext {
            kind: "yt".to_string(),
            id: hash_id(&format!("yt:{}", vid)),
            src: vid.to_string(),
            title: v["title"].as_str().unwrap_or("YouTube clip").to_string(),
            artist: v["channel"].as_str().or_else(|| v["uploader"].as_str()).unwrap_or("YouTube").replace(" - Topic", ""),
            album: String::new(),
            dur: v["duration"].as_f64().unwrap_or(0.0) as f32,
            cover: format!("https://i.ytimg.com/vi/{}/hqdefault.jpg", vid),
        });
    }
    if list.is_empty() {
        return Err(if out.status.success() { "nothing found at that link".to_string() } else { last_line(&out.stderr) });
    }
    if name.is_empty() {
        name = list_name(url);
    }
    Ok((name, list))
}

/// A SoundCloud user (name or profile link): their playlists as (title, link).
pub fn sc_sets(user: &str) -> Result<Vec<(String, String)>, String> {
    let exe = ytdlp_path().ok_or_else(|| "yt-dlp is not installed - press GET YT-DLP".to_string())?;
    let u = user.trim().trim_end_matches('/');
    let name = u.trim_start_matches("https://").trim_start_matches("http://").trim_start_matches("www.");
    let name = name.trim_start_matches("soundcloud.com/").trim_start_matches('@');
    let name = name.split('/').next().unwrap_or("");
    if name.is_empty() {
        return Err("type a SoundCloud user name or paste a profile link".to_string());
    }
    let url = format!("https://soundcloud.com/{}/sets", name);
    let out = command(&exe)
        .args(["--no-warnings", "--flat-playlist", "--dump-json", "--playlist-end", "60"])
        .args(cookie_args())
        .arg(&url)
        .timed("SoundCloud playlists")
        .map_err(|e| format!("could not run yt-dlp: {}", e))?;
    let mut list = Vec::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else { continue };
        let link = v["url"].as_str().or_else(|| v["webpage_url"].as_str()).unwrap_or("");
        if link.is_empty() {
            continue;
        }
        let title = v["title"].as_str().unwrap_or(link).to_string();
        list.push((title, link.to_string()));
    }
    if list.is_empty() {
        return Err(if out.status.success() {
            "no public playlists found for that user".to_string()
        } else {
            last_line(&out.stderr)
        });
    }
    Ok(list)
}

/// Title / uploader / length / thumbnail of a clip.
pub fn yt_info(vid: &str) -> Result<Ext, String> {
    let exe = ytdlp_path().ok_or_else(|| "yt-dlp is not installed - press GET YT-DLP".to_string())?;
    let url = media_url(vid);
    let out = command(&exe)
        .args(["--no-playlist", "--no-warnings", "--dump-single-json"])
        .args(if url.contains("soundcloud.com") { cookie_args() } else { Vec::new() })
        .arg(&url)
        .timed("reading a YouTube video")
        .map_err(|e| format!("could not run yt-dlp: {}", e))?;
    if !out.status.success() {
        return Err(last_line(&out.stderr));
    }
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
    let title = v["title"].as_str().unwrap_or("YouTube clip").to_string();
    let artist = v["artist"]
        .as_str()
        .or_else(|| v["uploader"].as_str())
        .or_else(|| v["channel"].as_str())
        .unwrap_or(if vid.starts_with("http") { "SoundCloud" } else { "YouTube" })
        .replace(" - Topic", "");
    Ok(Ext {
        kind: "yt".to_string(),
        id: hash_id(&format!("yt:{}", vid)),
        src: vid.to_string(),
        title,
        artist,
        album: String::new(),
        dur: v["duration"].as_f64().unwrap_or(0.0) as f32,
        cover: if vid.starts_with("http") {
            v["thumbnail"].as_str().unwrap_or("").to_string()
        } else {
            format!("https://i.ytimg.com/vi/{}/hqdefault.jpg", vid)
        },
    })
}

/// Download the audio of a clip (AAC in an m4a, which the player can decode) and return the bytes.
pub fn yt_fetch(src: &str) -> Result<Vec<u8>, String> {
    let exe = ytdlp_path().ok_or_else(|| "yt-dlp is not installed - press GET YT-DLP".to_string())?;
    let tmp = crate::api::config_dir().join("tmp");
    let _ = std::fs::create_dir_all(&tmp);
    let stem = file_stem(src);
    let vid = stem.as_str();
    if let Ok(rd) = std::fs::read_dir(&tmp) {
        for e in rd.flatten() {
            if e.file_name().to_string_lossy().starts_with(vid) {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    let url = media_url(src);
    let t0 = Instant::now();
    let mut child = command(&exe)
        .args(["--no-playlist", "--no-progress", "-f", "140/http_mp3_1_0/http_mp3_0_0/bestaudio[ext=mp3]/bestaudio[ext=m4a]/bestaudio"])
        .args(if url.contains("soundcloud.com") { cookie_args() } else { Vec::new() })
        .arg("-o")
        .arg(tmp.join(format!("{}.%(ext)s", vid)))
        .arg(&url)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("could not run yt-dlp: {}", e))?;
    // (what it says on stderr, read alongside, so a lot of it never holds yt-dlp up)
    let errs = child.stderr.take().map(|mut e| {
        std::thread::spawn(move || {
            let mut b = Vec::new();
            let _ = e.read_to_end(&mut b);
            b
        })
    });
    // when yt-dlp first says something (it has started) and when the download begins (it has found the audio)
    let (mut spoke, mut found) = (None, None);
    if let Some(out) = child.stdout.take() {
        for line in std::io::BufReader::new(out).lines().map_while(Result::ok) {
            spoke.get_or_insert(t0.elapsed());
            if found.is_none() && line.starts_with("[download]") {
                found = Some(t0.elapsed());
            }
        }
    }
    let status = child.wait().map_err(|e| format!("yt-dlp: {}", e))?;
    let stderr = errs.and_then(|h| h.join().ok()).unwrap_or_default();
    let at = |d: Option<Duration>| d.map_or("-".to_string(), |d| format!("{:.1} s", d.as_secs_f32()));
    let said: Vec<String> = String::from_utf8_lossy(&stderr)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .take(3)
        .map(|l| l.chars().take(160).collect())
        .collect();
    crate::api::log(&format!(
        "youtube {}: {} started after {}, found the audio after {}, done after {:.1} s{}",
        vid,
        build_name(&exe),
        at(spoke),
        at(found),
        t0.elapsed().as_secs_f32(),
        if said.is_empty() { String::new() } else { format!(" (it said: {})", said.join(" | ")) }
    ));
    if !status.success() {
        return Err(last_line(&stderr));
    }
    let mut found: Option<PathBuf> = None;
    if let Ok(rd) = std::fs::read_dir(&tmp) {
        for e in rd.flatten() {
            if e.file_name().to_string_lossy().starts_with(vid) {
                found = Some(e.path());
            }
        }
    }
    let path = found.ok_or_else(|| "yt-dlp finished but the file is missing".to_string())?;
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    let bytes = std::fs::read(&path).map_err(|e| e.to_string());
    let _ = std::fs::remove_file(&path);
    if ext == "webm" || ext == "opus" || ext == "ogg" {
        return Err("Only Opus audio was offered - press UPDATE YT-DLP and try again".to_string());
    }
    bytes
}

// --------------------------------------------------------------- analysis
/// The loudness envelope, 50 values per second (0..255), for drawing the waveform, and a fine outline for zooming in: the lowest and highest sample of each
/// millisecond, scaled so the loudest moment reaches the edge.
pub fn wave_data(bytes: Vec<u8>) -> Option<(Vec<u8>, Vec<[i8; 2]>)> {
    let src = SymSource::new(bytes).ok()?;
    let ch = src.channels().max(1) as usize;
    let per = (src.sample_rate() / 50).max(1) as usize;
    let rate = src.sample_rate().max(1) as u64;
    let mut out = Vec::new();
    let mut fine: Vec<[i16; 2]> = Vec::new();
    let (mut peak, mut n, mut c) = (0i32, 0usize, 0usize);
    // (milliseconds counted from the frame number, so a 44.1 kHz song doesn't drift)
    let (mut lo, mut hi, mut frame) = (0i16, 0i16, 0u64);
    for s in src {
        peak = peak.max((s as i32).abs());
        lo = lo.min(s);
        hi = hi.max(s);
        c += 1;
        if c == ch {
            c = 0;
            n += 1;
            frame += 1;
            if frame * 1000 / rate > fine.len() as u64 {
                fine.push([lo, hi]);
                lo = 0;
                hi = 0;
            }
            if n == per {
                let v = (peak as f32 / 32768.0).clamp(0.0, 1.0).powf(0.6);
                out.push((v * 255.0) as u8);
                peak = 0;
                n = 0;
            }
        }
    }
    if out.is_empty() {
        return None;
    }
    let top = fine.iter().map(|p| (p[0] as i32).abs().max((p[1] as i32).abs())).max().unwrap_or(1).max(1) as f32;
    let scale = |v: i16| {
        let f = v as f32 / top;
        (f.signum() * f.abs().powf(0.8) * 127.0) as i8
    };
    Some((out, fine.into_iter().map(|p| [scale(p[0]), scale(p[1])]).collect()))
}

/// Samples from `a` to `b` seconds of an encoded song (all of it with `b` at 0), with its channels and rate.
pub fn decode_span(bytes: Vec<u8>, a: f32, b: f32) -> Result<(Vec<i16>, u16, u32), String> {
    let mut src = SymSource::new(bytes)?;
    let (ch, rate) = (src.channels(), src.sample_rate());
    if a > 0.0 {
        src.try_seek(Duration::from_secs_f32(a)).map_err(|_| "can't seek in this file".to_string())?;
    }
    let data: Vec<i16> = if b > a { src.take(((b - a) * rate as f32) as usize * ch as usize).collect() } else { src.collect() };
    if data.is_empty() {
        return Err("nothing to export".to_string());
    }
    Ok((data, ch, rate))
}

/// The stems that are on, mixed, from `a` to `b` seconds (to the end with `b` at 0): stereo at 44.1 kHz.
pub fn mix_stems_span(data: &[Vec<i16>; 4], on: [bool; 4], a: f32, b: f32) -> Vec<i16> {
    let len = data.iter().map(|d| d.len()).max().unwrap_or(0);
    let s0 = ((a.max(0.0) * 44100.0) as usize * 2).min(len);
    let s1 = if b > a { ((b * 44100.0) as usize * 2).min(len) } else { len };
    (s0..s1.max(s0))
        .map(|i| {
            let v: i32 = (0..4).filter(|k| on[*k]).map(|k| data[k].get(i).copied().unwrap_or(0) as i32).sum();
            v.clamp(i16::MIN as i32, i16::MAX as i32) as i16
        })
        .collect()
}

/// A 5 ms fade in and out, so a file cut from the middle of a song doesn't click at either end.
pub fn fade_ends(data: &mut [i16], ch: u16, rate: u32) {
    let ch = ch.max(1) as usize;
    let frames = data.len() / ch;
    let n = (rate as usize / 200).min(frames / 2);
    for f in 0..n {
        let g = f as f32 / n as f32;
        for c in 0..ch {
            for i in [f * ch + c, (frames - 1 - f) * ch + c] {
                data[i] = (data[i] as f32 * g) as i16;
            }
        }
    }
}

/// Write 16-bit samples as a WAV file.
pub fn write_wav(path: &Path, ch: u16, rate: u32, data: &[i16]) -> Result<(), String> {
    if data.is_empty() {
        return Err("nothing to export".to_string());
    }
    if let Some(d) = path.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let len = (data.len() * 2) as u32;
    let mut out: Vec<u8> = Vec::with_capacity(44 + len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&ch.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * ch as u32 * 2).to_le_bytes());
    out.extend_from_slice(&(ch * 2).to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&len.to_le_bytes());
    for s in data {
        out.extend_from_slice(&s.to_le_bytes());
    }
    std::fs::write(path, out).map_err(|e| e.to_string())
}

// ------------------------------------------------------------ tune lookup
fn urlenc(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            o.push(b as char);
        } else {
            o.push_str(&format!("%{:02X}", b));
        }
    }
    o
}

/// What's known about a tune: its title, who wrote it, and the year it came out.
#[derive(Clone, Default, Debug)]
pub struct Work {
    pub title: String,
    /// "Cole Porter" or "Harry Warren, Mack Gordon (words)"; empty when none is found
    pub writers: String,
    /// the year it was written or first published (never a later recording's or a reissue's)
    pub year: String,
}

/// A title the way two spellings of it compare: letters and digits only ("So Near, So Far" = "so near so far").
pub fn title_key(s: &str) -> String {
    s.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect()
}

/// `title_key` without "(song)" and the like, and without a leading "The" or "A".
fn bare_title_key(s: &str) -> String {
    let mut t = String::new();
    let mut depth = 0;
    for c in s.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth = (depth - 1).max(0),
            _ if depth == 0 => t.push(c),
            _ => {}
        }
    }
    let low = t.trim().to_lowercase();
    let low = low.strip_prefix("the ").or_else(|| low.strip_prefix("a ")).unwrap_or(&low);
    title_key(low)
}

/// A composer as the standards list or an iReal link writes it, the way people read it: "Jimmy Van-Heusen" is
/// "Jimmy Van Heusen", a team of last names ("Raye-DePaul") is "Raye, DePaul", and with `last_first` (iReal's own
/// order, "Warren Harry") the two names are turned round.
pub fn tidy_composer(s: &str, last_first: bool) -> String {
    let s = s.trim();
    if !s.contains(' ') {
        return s.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join(", ");
    }
    let mut words: Vec<String> = s.split_whitespace().map(|w| w.replace('-', " ")).collect();
    let suffix = |w: &str| matches!(w.trim_end_matches('.').to_lowercase().as_str(), "ii" | "iii" | "jr" | "sr");
    if last_first && words.len() == 2 && !suffix(&words[1]) {
        words.swap(0, 1);
    }
    words.join(" ")
}

/// The words in names worth matching on ("Sonny Rollins Trio" -> sonny, rollins): four letters or more, no band words.
fn name_words(s: &str) -> Vec<String> {
    const SKIP: [&str; 9] = ["trio", "quartet", "quintet", "sextet", "band", "orchestra", "group", "with", "featuring"];
    s.split(|c: char| !c.is_alphanumeric())
        .map(|w| w.to_lowercase())
        .filter(|w| w.chars().count() >= 4 && !SKIP.contains(&w.as_str()))
        .collect()
}

/// A song as Wikidata has it (names already read from their own entries).
#[derive(Clone, Default, Debug)]
struct Song {
    label: String,
    about: String,
    composers: Vec<String>,
    lyricists: Vec<String>,
    performers: Vec<String>,
    year: String,
}

impl Song {
    /// "Harry Warren, Mack Gordon (words)": the composers, then whoever wrote only the words.
    fn writers(&self) -> String {
        let mut out: Vec<String> = Vec::new();
        for c in &self.composers {
            if !out.contains(c) {
                out.push(c.clone());
            }
        }
        for l in &self.lyricists {
            if !self.composers.contains(l) && !out.contains(&format!("{} (words)", l)) {
                out.push(format!("{} (words)", l));
            }
        }
        out.join(", ")
    }
}

/// The year a description starts with ("1960 composition by John Coltrane").
fn lead_year(about: &str) -> Option<String> {
    let y: String = about.chars().take_while(|c| c.is_ascii_digit()).collect();
    (y.len() == 4 && about[4..].starts_with(' ') && ("1600".."2100").contains(&y.as_str())).then_some(y)
}

/// Of the songs found, the one with this title that the hints point to: a writer's or a player's name in its
/// credits or its description. None when nothing points anywhere, since another song's details are worse than none.
fn pick_song<'a>(songs: &'a [Song], name: &str, hints: &[String]) -> Option<&'a Song> {
    let want = bare_title_key(name);
    let text = |s: &Song| {
        title_key(&format!("{} {} {} {}", s.about, s.composers.join(" "), s.lyricists.join(" "), s.performers.join(" ")))
    };
    let mut hit: Vec<&Song> = songs
        .iter()
        .filter(|s| bare_title_key(&s.label) == want)
        .filter(|s| {
            let t = text(s);
            hints.iter().any(|h| t.contains(h.as_str()))
        })
        .collect();
    // the fullest first (named writers, then a year); otherwise the search's order
    hit.sort_by_key(|s| (s.composers.is_empty() && s.lyricists.is_empty(), s.year.is_empty()));
    hit.first().copied()
}

/// A client for the lookups, and a GET that reads JSON.
fn json_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent("Backline/0.7 (personal practice app)")
        .build()
        .map_err(|e| e.to_string())
}

/// Asked too often ("too many requests", or MusicBrainz's "unavailable"), it waits as long as it's told (a few
/// seconds at most) and asks again, twice.
fn get_json(client: &reqwest::blocking::Client, url: &str) -> Result<serde_json::Value, String> {
    crate::api::guard()?;
    let mut tries = 0;
    loop {
        let r = client.get(url).send().map_err(|e| e.to_string())?;
        let busy = matches!(r.status().as_u16(), 429 | 503);
        if busy && tries < 2 {
            tries += 1;
            let wait = r.headers().get("retry-after").and_then(|v| v.to_str().ok()?.parse::<u64>().ok()).unwrap_or(2).clamp(1, 5);
            std::thread::sleep(Duration::from_secs(wait));
            continue;
        }
        return r.error_for_status().map_err(|e| e.to_string())?.json::<serde_json::Value>().map_err(|e| e.to_string());
    }
}

/// The song in Wikidata (songs and compositions only) that the hints point to, with its writers and year.
fn wikidata_song(client: &reqwest::blocking::Client, name: &str, hints: &[String]) -> Result<Option<Work>, String> {
    const API: &str = "https://www.wikidata.org/w/api.php";
    if hints.is_empty() {
        return Ok(None);
    }
    let pause = || std::thread::sleep(Duration::from_millis(700));
    // song, musical work / composition
    let q = format!("{} haswbstatement:P31=Q7366|P31=Q105543609|P31=Q2188189|P31=Q207628", name);
    let v = get_json(client, &format!("{API}?action=query&list=search&srsearch={}&srlimit=10&format=json", urlenc(&q)))?;
    let ids: Vec<String> = v["query"]["search"]
        .as_array()
        .map(|a| a.iter().filter_map(|h| h["title"].as_str().map(String::from)).collect())
        .unwrap_or_default();
    if ids.is_empty() {
        return Ok(None);
    }
    pause();
    let e = get_json(
        client,
        &format!("{API}?action=wbgetentities&ids={}&props={}&languages=en&format=json", urlenc(&ids.join("|")), urlenc("claims|labels|descriptions")),
    )?;
    let items: Vec<&serde_json::Value> = ids.iter().filter_map(|id| e["entities"].get(id)).collect();
    let refs = |it: &serde_json::Value, p: &str| -> Vec<String> {
        it["claims"][p]
            .as_array()
            .map(|a| a.iter().filter_map(|c| c["mainsnak"]["datavalue"]["value"]["id"].as_str().map(String::from)).collect())
            .unwrap_or_default()
    };
    let years = |it: &serde_json::Value, p: &str| -> Vec<String> {
        it["claims"][p]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|c| c["mainsnak"]["datavalue"]["value"]["time"].as_str())
                    .filter(|t| t.starts_with('+') && t.len() >= 5)
                    .map(|t| t[1..5].to_string())
                    .collect()
            })
            .unwrap_or_default()
    };
    // everyone named, in one more request
    let mut people: Vec<String> = Vec::new();
    for &it in &items {
        for p in ["P86", "P676", "P175"] {
            for id in refs(it, p) {
                if !people.contains(&id) && people.len() < 50 {
                    people.push(id);
                }
            }
        }
    }
    let names = if people.is_empty() {
        serde_json::Value::Null
    } else {
        pause();
        get_json(client, &format!("{API}?action=wbgetentities&ids={}&props=labels&languages=en&format=json", urlenc(&people.join("|"))))?
    };
    let name_of = |id: &String| names["entities"][id]["labels"]["en"]["value"].as_str().map(String::from);
    let songs: Vec<Song> = items
        .iter()
        .map(|&it| {
            let about = it["descriptions"]["en"]["value"].as_str().unwrap_or("").to_string();
            let mut ys = years(it, "P577");
            if ys.is_empty() {
                ys = years(it, "P571");
            }
            ys.sort();
            Song {
                label: it["labels"]["en"]["value"].as_str().unwrap_or("").to_string(),
                composers: refs(it, "P86").iter().filter_map(name_of).collect(),
                lyricists: refs(it, "P676").iter().filter_map(name_of).collect(),
                performers: refs(it, "P175").iter().filter_map(name_of).collect(),
                year: ys.first().cloned().or_else(|| lead_year(&about)).unwrap_or_default(),
                about,
            }
        })
        .collect();
    Ok(pick_song(&songs, name, hints).map(|s| Work { title: s.label.clone(), writers: s.writers(), year: s.year.clone() }))
}

/// Who wrote a tune and when, from MusicBrainz (free, no key). With `artist` (someone who recorded it) the song is
/// found through that recording, so a title other songs share still finds the right one; without, only a title no
/// other song has. The year is the one given for the writing, else the recording's when it's the writer's own.
fn musicbrainz_work(client: &reqwest::blocking::Client, name: &str, artist: Option<&str>) -> Result<Work, String> {
    let get = |url: String| get_json(client, &url);
    let want = title_key(name);
    let pause = || std::thread::sleep(Duration::from_millis(1100)); // MusicBrainz asks for one request a second
    let artist = artist.map(str::trim).filter(|a| !a.is_empty());
    // through a recording by someone who played it: that recording's song
    let mut via: Option<(String, String)> = None; // (work id, the recording's year)
    if let Some(a) = artist {
        let q = format!("recording:\"{}\" AND artist:\"{}\"", name, a);
        let v = get(format!("https://musicbrainz.org/ws/2/recording/?query={}&fmt=json&limit=10", urlenc(&q)))?;
        let recs = v["recordings"].as_array().cloned().unwrap_or_default();
        let same: Vec<&serde_json::Value> = recs.iter().filter(|r| r["title"].as_str().is_some_and(|t| title_key(t) == want)).collect();
        // the year of their earliest release of it (later ones are reissues and compilations)
        let year = same
            .iter()
            .filter_map(|r| r["first-release-date"].as_str().filter(|d| d.len() >= 4).map(|d| d[..4].to_string()))
            .min()
            .unwrap_or_default();
        for r in same.iter().take(3) {
            let Some(rid) = r["id"].as_str() else { continue };
            pause();
            let rv = get(format!("https://musicbrainz.org/ws/2/recording/{}?inc=work-rels&fmt=json", rid))?;
            let work = rv["relations"].as_array().and_then(|a| a.iter().find_map(|x| x["work"]["id"].as_str().map(String::from)));
            if let Some(w) = work {
                via = Some((w, year.clone()));
                break;
            }
        }
        // their recording is there but not tied to a song: better nothing than another song's details
        if via.is_none() && !same.is_empty() {
            return Ok(Work { title: name.to_string(), ..Default::default() });
        }
        pause();
    }
    let (id, rec_year) = match via {
        Some(v) => v,
        None => {
            let q = format!("work:\"{}\"", name);
            let v = get(format!("https://musicbrainz.org/ws/2/work/?query={}&fmt=json&limit=10", urlenc(&q)))?;
            let works = v["works"].as_array().cloned().unwrap_or_default();
            let same: Vec<&serde_json::Value> = works.iter().filter(|w| w["title"].as_str().is_some_and(|t| title_key(t) == want)).collect();
            match same.len() {
                0 => return Err("no song with that title".to_string()),
                1 => (same[0]["id"].as_str().ok_or_else(|| "no work id".to_string())?.to_string(), String::new()),
                _ => return Err("several songs have that title".to_string()),
            }
        }
    };
    pause();
    let w = get(format!("https://musicbrainz.org/ws/2/work/{}?inc=artist-rels&fmt=json", id))?;
    let mut writers: Vec<String> = Vec::new();
    let mut years: Vec<String> = Vec::new();
    for r in w["relations"].as_array().cloned().unwrap_or_default() {
        let t = r["type"].as_str().unwrap_or("");
        if t == "composer" || t == "writer" || t == "lyricist" {
            if let Some(n) = r["artist"]["name"].as_str() {
                // someone who wrote the music and the words is named once
                let tag = if t == "lyricist" { format!("{} (words)", n) } else { n.to_string() };
                let music_too = t == "lyricist" && writers.iter().any(|w| w == n);
                if !writers.contains(&tag) && !music_too {
                    if t != "lyricist" {
                        writers.retain(|w| *w != format!("{} (words)", n));
                    }
                    writers.push(tag);
                }
            }
            if let Some(y) = r["begin"].as_str().filter(|y| y.len() >= 4) {
                years.push(y[..4].to_string());
            }
        }
    }
    // no year for the writing: the recording's, only when it's the writer's own (anyone else's came later)
    if years.is_empty() && !rec_year.is_empty() {
        let own = artist.map(name_words).unwrap_or_default();
        if writers.iter().any(|w| own.iter().any(|x| title_key(w).contains(x.as_str()))) {
            years.push(rec_year);
        }
    }
    years.sort();
    let title = w["title"].as_str().unwrap_or(name).to_string();
    Ok(Work { title, writers: writers.join(", "), year: years.first().cloned().unwrap_or_default() })
}

/// Who wrote a tune and when it came out. Wikidata first: it has the composer, the lyricist and the year a
/// song came out for most standards, and the song is picked by a name it should carry (the writer in the jazz
/// standards list, or the player of your recording). Then MusicBrainz, through your recording. The standards
/// list's composer covers what neither knows. A year stays empty rather than being a later recording's.
pub fn lookup_work(name: &str, artist: Option<&str>) -> Result<Work, String> {
    let client = json_client()?;
    let listed = listed_composer(name);
    let mut hints: Vec<String> = Vec::new();
    for n in listed.iter().map(String::as_str).chain(artist) {
        hints.extend(name_words(n));
    }
    // the standards list's writer, when the other answer isn't plainly the same people
    let agrees = |writers: &str| listed.as_ref().is_none_or(|l| name_words(l).iter().any(|x| title_key(writers).contains(x.as_str())));
    if let Ok(Some(mut w)) = wikidata_song(&client, name, &hints) {
        if w.writers.is_empty() {
            w.writers = listed.clone().unwrap_or_default();
        }
        if w.year.is_empty() || w.writers.is_empty() {
            if let Ok(m) = musicbrainz_work(&client, name, artist) {
                if !m.writers.is_empty() && agrees(&m.writers) {
                    if w.writers.is_empty() {
                        w.writers = m.writers;
                    }
                    if w.year.is_empty() {
                        w.year = m.year;
                    }
                }
            }
        }
        return Ok(w);
    }
    match musicbrainz_work(&client, name, artist) {
        Ok(m) if !m.writers.is_empty() && agrees(&m.writers) => Ok(m),
        Ok(m) => Ok(Work { title: m.title, writers: listed.clone().unwrap_or_default(), year: String::new() }),
        Err(_) if listed.is_some() => Ok(Work { title: name.to_string(), writers: listed.unwrap_or_default(), year: String::new() }),
        Err(e) => Err(e),
    }
}

/// The open Jazz Standards data (about 1,300 songs), downloaded once and kept next to the other app data.
fn standards() -> Result<Vec<serde_json::Value>, String> {
    let path = crate::api::config_dir().join("charts.json");
    if !path.exists() {
        crate::api::guard()?;
        let client = reqwest::blocking::Client::builder().timeout(Duration::from_secs(40)).build().map_err(|e| e.to_string())?;
        let body = client
            .get("https://raw.githubusercontent.com/mikeoliphant/JazzStandards/main/JazzStandards.json")
            .send()
            .and_then(|r| r.error_for_status())
            .and_then(|r| r.text())
            .map_err(|e| format!("couldn't reach the chart list ({})", e))?;
        let _ = std::fs::create_dir_all(crate::api::config_dir());
        std::fs::write(&path, body).map_err(|e| e.to_string())?;
    }
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| {
        let _ = std::fs::remove_file(&path);
        e.to_string()
    })
}

/// Who the standards list says wrote a tune with exactly this title.
fn listed_composer(name: &str) -> Option<String> {
    let list = standards().ok()?;
    let want = bare_title_key(name);
    let song = list.iter().find(|v| v["Title"].as_str().is_some_and(|t| bare_title_key(t) == want))?;
    Some(tidy_composer(song["Composer"].as_str()?, false)).filter(|c| !c.is_empty())
}

/// Chord changes for a tune from the open Jazz Standards data (about 1,300 songs).
/// The file is downloaded once and kept next to the other app data. On a miss the error
/// carries the closest titles, for "did you mean" buttons.
pub fn find_chart(name: &str) -> Result<(String, String, String), (String, Vec<String>)> {
    let list = standards().map_err(|e| (e, Vec::new()))?;
    match crate::chart::find_standard(&list, name) {
        Some(song) => crate::chart::from_standard(song)
            .map(|(chart, key, composer)| (chart, key, tidy_composer(&composer, false)))
            .ok_or_else(|| ("that chart has no chords".to_string(), Vec::new())),
        None => Err(("not in the free chart list (jazz standards only)".to_string(), crate::chart::suggest(&list, name, 5))),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn explorer_is_given_the_path_in_quotes() {
        assert_eq!(super::explorer_select("D:/Music/My Song (live).mp3"), r#"/select,"D:\Music\My Song (live).mp3""#);
        assert_eq!(super::explorer_select(r"\\?\C:\Users\me\a b.wav"), r#"/select,"C:\Users\me\a b.wav""#);
    }

    /// Fetching a YouTube song logs how long each part took (run by hand: BACKLINE_HOME with yt-dlp in its bin folder).
    #[test]
    #[ignore]
    fn yt_fetch_logs_how_long_each_part_took() {
        let b = yt_fetch("aqz-KE-bpKQ").expect("fetched");
        assert!(b.len() > 1_000_000);
        let log = std::fs::read_to_string(crate::api::config_dir().join("log.txt")).unwrap();
        let line = log.lines().rev().find(|l| l.contains("youtube aqz-KE-bpKQ")).unwrap().to_string();
        println!("LOGGED {}", line);
        assert!(line.contains("found the audio after") && line.contains("done after"));
    }

    use super::*;

    #[test]
    fn fine_wave_shows_each_attack() {
        // one second of stereo silence with a short note starting at half a second
        let n = 44100usize;
        let mut data = Vec::new();
        for i in 0..n {
            let t = i as f32 / 44100.0;
            let v = if (0.5..0.52).contains(&t) { (16000.0 * (t * 440.0 * std::f32::consts::TAU).sin()) as i16 } else { 0 };
            data.extend_from_slice(&v.to_le_bytes());
            data.extend_from_slice(&v.to_le_bytes());
        }
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        for v in [16u32] { wav.extend_from_slice(&v.to_le_bytes()); }
        for v in [1u16, 2] { wav.extend_from_slice(&v.to_le_bytes()); }
        for v in [44100u32, 44100 * 4] { wav.extend_from_slice(&v.to_le_bytes()); }
        for v in [4u16, 16] { wav.extend_from_slice(&v.to_le_bytes()); }
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&(data.len() as u32).to_le_bytes());
        wav.extend_from_slice(&data);
        let (coarse, fine) = wave_data(wav).expect("a wave");
        assert_eq!(coarse.len(), 50);
        assert_eq!(fine.len(), 1000);
        // silent before, full height during the note, silent again after
        assert_eq!(fine[400], [0, 0]);
        let (lo, hi) = fine[502..510].iter().fold((0, 0), |(l, h), p| (p[0].min(l), p[1].max(h)));
        assert!(hi >= 120 && lo <= -120, "{} {}", lo, hi);
        assert_eq!(fine[600], [0, 0]);
    }

    #[test]
    fn youtube_ids_from_any_link() {
        let id = Some("dQw4w9WgXcQ".to_string());
        for link in [
            "dQw4w9WgXcQ",
            "  dQw4w9WgXcQ  ",
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
            "https://www.youtube.com/watch?list=PL123&v=dQw4w9WgXcQ&t=42s",
            "https://youtu.be/dQw4w9WgXcQ?si=abc",
            "https://www.youtube.com/shorts/dQw4w9WgXcQ",
            "https://www.youtube.com/embed/dQw4w9WgXcQ",
            "https://www.youtube.com/live/dQw4w9WgXcQ",
        ] {
            assert_eq!(yt_video_id(link), id, "{}", link);
        }
        for bad in ["", "hello", "https://www.youtube.com/watch?v=short", "https://example.com/x", "dQw4w9WgXc!"] {
            assert_eq!(yt_video_id(bad), None, "{}", bad);
        }
    }

    #[test]
    fn playlist_links_and_names() {
        assert!(is_yt_list("https://www.youtube.com/playlist?list=PLabc"));
        // a video played from a playlist reads the whole playlist
        let v = "https://www.youtube.com/watch?v=YYC5enmW69o&list=PLVbKYLGZgn8GIHg6A-3tW-v06arFGWdXy";
        assert!(is_yt_list(v));
        assert_eq!(yt_list_url(v), "https://www.youtube.com/playlist?list=PLVbKYLGZgn8GIHg6A-3tW-v06arFGWdXy");
        assert!(is_yt_list("https://www.youtube.com/watch?list=PLabc&v=dQw4w9WgXcQ&t=42s"));
        // ...but one played from a Mix (made up by YouTube around it) is that one video
        assert!(!is_yt_list("https://www.youtube.com/watch?v=dQw4w9WgXcQ&list=RDdQw4w9WgXcQ&start_radio=1"));
        assert!(is_yt_list("https://www.youtube.com/playlist?list=RDCLAK5uy_abc"));
        assert!(!is_yt_list("https://www.youtube.com/watch?v=dQw4w9WgXcQ"));
        assert!(!is_yt_list("https://www.youtube.com/watch?v=dQw4w9WgXcQ&playlist=x"));
        assert_eq!(list_name("https://soundcloud.com/someone/sets/my-cool_mix?si=1"), "My Cool Mix");
        assert_eq!(list_name("https://soundcloud.com/someone/sets/late-night/"), "Late Night");
        assert_eq!(list_name("https://soundcloud.com/---"), "Playlist");
    }

    #[test]
    fn soundcloud_names_from_the_link() {
        assert_eq!(
            slug_meta("https://soundcloud.com/some-artist/cool-track?in=x"),
            Some(("Some Artist".to_string(), "Cool Track".to_string()))
        );
        assert_eq!(slug_meta("https://m.soundcloud.com/a/b"), Some(("A".to_string(), "B".to_string())));
        // playlists, likes and API links are not a single track
        assert_eq!(slug_meta("https://soundcloud.com/someone/sets/a-mix"), None);
        assert_eq!(slug_meta("https://soundcloud.com/someone/likes"), None);
        assert_eq!(slug_meta("https://api.soundcloud.com/tracks/1"), None);
        assert_eq!(slug_meta("https://youtube.com/a/b"), None);
        assert_eq!(slug_meta("https://soundcloud.com/only-artist"), None);
    }

    #[test]
    fn soundcloud_listing() {
        let lines = concat!(
            r#"{"url": "https://soundcloud.com/dj-x/first-song", "duration": 200.5}"#,
            "\n",
            r#"{"webpage_url": "https://soundcloud.com/dj-x/second", "title": "Real Title", "uploader": "DJ X", "thumbnail": "http://t"}"#,
            "\n",
            "not json\n",
            r#"{"url": "https://soundcloud.com/dj-x/first-song"}"#,
            "\n",
            r#"{"url": "https://youtube.com/watch?v=1"}"#,
        );
        let e = parse_entries(lines.as_bytes());
        assert_eq!(e.len(), 2, "duplicates, junk and other sites are skipped");
        // no title in a quick listing: borrowed from the link
        assert_eq!((e[0].title.as_str(), e[0].artist.as_str(), e[0].dur), ("First Song", "Dj X", 200.5));
        assert_eq!((e[1].title.as_str(), e[1].artist.as_str(), e[1].cover.as_str()), ("Real Title", "DJ X", "http://t"));
        assert_eq!(e[0].id, hash_id("yt:https://soundcloud.com/dj-x/first-song"));
    }

    #[test]
    fn download_names() {
        assert_eq!(media_url("dQw4w9WgXcQ"), "https://www.youtube.com/watch?v=dQw4w9WgXcQ");
        assert_eq!(media_url("https://soundcloud.com/a/b"), "https://soundcloud.com/a/b");
        assert_eq!(file_stem("dQw4w9WgXcQ"), "dQw4w9WgXcQ");
        let s = file_stem("https://soundcloud.com/a/b");
        assert!(s.starts_with("sc") && s[2..].chars().all(|c| c.is_ascii_digit()), "{}", s);
        assert_eq!(last_line(b"line one\nERROR: it broke\n\n  "), "ERROR: it broke");
        assert_eq!(last_line(b""), "unknown error");
    }

    #[test]
    fn audio_files() {
        assert!(is_audio(Path::new("a/b/song.FLAC")));
        assert!(is_audio(Path::new("x.m4a")));
        assert!(!is_audio(Path::new("cover.jpg")));
        assert!(!is_audio(Path::new("noext")));
    }
}

#[cfg(test)]
mod playlist_file_tests {
    use super::*;

    #[test]
    fn reads_m3u_and_pls() {
        let dir = std::env::temp_dir().join("backline_pl_test");
        let _ = std::fs::create_dir_all(dir.join("sub"));
        std::fs::write(dir.join("one.mp3"), b"x").unwrap();
        std::fs::write(dir.join("sub").join("two song.flac"), b"x").unwrap();
        let abs = dir.join("one.mp3");
        let m3u = format!(
            "#EXTM3U
#EXTINF:123,One
one.mp3
sub/two%20song.flac
file:///{}
missing.mp3
notes.txt
",
            abs.to_string_lossy().replace('\\', "/")
        );
        std::fs::write(dir.join("set.m3u"), m3u).unwrap();
        let (name, songs) = read_playlist_file(&dir.join("set.m3u")).unwrap();
        assert_eq!(name, "set");
        assert_eq!(songs.len(), 3, "{:?}", songs);
        assert!(songs[1].ends_with("two song.flac"));
        std::fs::write(
            dir.join("gig.pls"),
            "[playlist]
File1=one.mp3
Title1=One
NumberOfEntries=1
",
        )
        .unwrap();
        let (name, songs) = read_playlist_file(&dir.join("gig.pls")).unwrap();
        assert_eq!((name.as_str(), songs.len()), ("gig", 1));
        assert!(is_playlist_file(Path::new("a.M3U8")) && !is_playlist_file(Path::new("a.mp3")));
    }
}

/// Checks that need the internet and a real machine - run by the Mac test on GitHub (`-- --ignored smoke`).
#[cfg(test)]
mod smoke {
    use super::*;

    #[test]
    #[ignore]
    fn smoke_ytdlp_downloads_starts_fast_and_reads_a_playlist() {
        let exe = ytdlp_download().expect("yt-dlp download");
        println!("yt-dlp at {}", exe.display());
        // a second start is the one you wait for every time: it must be quick
        let _ = command(&exe).arg("--version").output();
        let t0 = std::time::Instant::now();
        let out = command(&exe).arg("--version").output().expect("yt-dlp runs");
        let secs = t0.elapsed().as_secs_f32();
        println!("yt-dlp {} started in {:.2} s", String::from_utf8_lossy(&out.stdout).trim(), secs);
        assert!(out.status.success(), "yt-dlp --version failed");
        assert!(secs < 3.0, "yt-dlp takes {:.1} s to start", secs);
        // a playlist (YouTube may turn a cloud machine away: reported, not failed)
        let t0 = std::time::Instant::now();
        match yt_list("https://www.youtube.com/playlist?list=PLFgquLnL59alCl_2TQvOiD5Vgm1hCaGSI") {
            Ok((name, list)) => println!("playlist '{}': {} videos in {:.1} s", name, list.len(), t0.elapsed().as_secs_f32()),
            Err(e) => println!("playlist not read here: {}", e),
        }
    }
}

#[cfg(test)]
mod details_tests {
    use super::*;

    #[test]
    fn composers_read_the_way_people_write_them() {
        assert_eq!(tidy_composer("Jimmy Van-Heusen", false), "Jimmy Van Heusen");
        assert_eq!(tidy_composer("Raye-DePaul", false), "Raye, DePaul");
        assert_eq!(tidy_composer("Antonio-Carlos Jobim", false), "Antonio Carlos Jobim");
        assert_eq!(tidy_composer("Traditional", false), "Traditional");
        // iReal's own order, last name first
        assert_eq!(tidy_composer("Warren Harry", true), "Harry Warren");
        assert_eq!(tidy_composer("Van-Heusen Jimmy", true), "Jimmy Van Heusen");
        assert_eq!(tidy_composer("Rodgers-Hammerstein II", true), "Rodgers Hammerstein II");
    }

    #[test]
    fn titles_match_whatever_the_punctuation() {
        assert_eq!(title_key("So Near, So Far"), title_key("so near so far"));
        assert_eq!(bare_title_key("Cherokee (song)"), "cherokee");
        assert_eq!(bare_title_key("The Song Is You"), bare_title_key("Song Is You"));
        assert_eq!(lead_year("1960 composition by John Coltrane").as_deref(), Some("1960"));
        assert_eq!(lead_year("song by Gomez"), None);
        assert_eq!(lead_year("19605 things"), None);
    }

    fn song(label: &str, about: &str, composers: &[&str], year: &str) -> Song {
        Song {
            label: label.into(),
            about: about.into(),
            composers: composers.iter().map(|c| c.to_string()).collect(),
            year: year.into(),
            ..Default::default()
        }
    }

    #[test]
    fn the_song_the_names_point_to_is_picked() {
        let found = vec![
            song("Countdown", "2013 song by Hardwell and MAKJ", &[], "2013"),
            song("Countdown", "1960 composition by John Coltrane", &[], "1960"),
            song("The Final Countdown", "song by Europe", &["Joey Tempest"], "1986"),
        ];
        assert_eq!(pick_song(&found, "Countdown", &name_words("John Coltrane")).map(|s| s.year.as_str()), Some("1960"));
        // a song of the same name by other people is no answer
        let gomez = vec![song("See the World", "song by Gomez", &["Ian Ball", "Tom Gray"], "2006")];
        assert!(pick_song(&gomez, "See The World", &name_words("Pat Metheny")).is_none());
        // the original over a later version of it
        let mut first = song("There Will Never Be Another You", "original song", &["Harry Warren"], "1942");
        first.performers = vec!["Chet Baker".into()];
        let found = vec![song("There Will Never Be Another You", "1978 live album by Sonny Rollins", &[], "1978"), first];
        let hints: Vec<String> = name_words("Harry Warren").into_iter().chain(name_words("Sonny Rollins")).collect();
        assert_eq!(pick_song(&found, "There Will Never Be Another You", &hints).map(|s| s.year.as_str()), Some("1942"));
    }

    #[test]
    fn writers_name_the_words_apart() {
        let mut s = song("There Will Never Be Another You", "", &["Harry Warren"], "1942");
        s.lyricists = vec!["Mack Gordon".into()];
        assert_eq!(s.writers(), "Harry Warren, Mack Gordon (words)");
        let mut c = song("Corcovado", "", &["Antônio Carlos Jobim"], "1960");
        c.lyricists = vec!["Antônio Carlos Jobim".into()];
        assert_eq!(c.writers(), "Antônio Carlos Jobim");
    }

    /// What the lookup finds for tunes that came back wrong before (run by hand: it asks Wikidata and MusicBrainz).
    #[test]
    #[ignore]
    fn tune_details_from_the_web() {
        for (name, artist) in [
            ("There Will Never Be Another You", Some("Joe Pass")),
            ("Cherokee", Some("Sonny Stitt")),
            ("Billie's Bounce", None),
            ("I'll Remember April", Some("Bud Powell")),
            ("so near so far", Some("Joe Henderson")),
            ("See The World", Some("Pat Metheny")),
            ("Corcovado", Some("Joe Pass")),
            ("Oleo", Some("Sonny Rollins Trio")),
            ("It Could Happen to You", None),
            ("Joy Spring", None),
            ("Countdown", None),
            ("Inner Urge", Some("Joe Henderson")),
            ("In Walked Bud", Some("Thelonious Monk")),
            ("All the Things You Are", None),
            ("Incompatibilidade de Gênios", Some("João Bosco")),
            ("He Can Handle It", Some("Crystal Aikin")),
        ] {
            let r = lookup_work(name, artist);
            println!("DETAILS {name} | {:?}", r.map(|w| (w.writers, w.year)));
            std::thread::sleep(Duration::from_secs(4));
        }
    }
}
