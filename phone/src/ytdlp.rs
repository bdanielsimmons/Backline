//! yt-dlp on the phone, as YTDLnis and Seal run it: Python 3.12 and QuickJS built for Android (from youtubedl-android)
//! and yt-dlp itself are packed in the app as lib/<abi>/lib*.so (`fetch_libs.sh`), which Android unpacks on install
//! into the app's library folder, the one place an app may run programs from. Python's own files (its standard
//! library, OpenSSL...) come zipped as libpython.zip.so and are unpacked into the app's storage the first time.
//! yt-dlp is copied out beside them so it can update itself (`-U`, once a day). On the PC copy, the desktop's
//! yt-dlp.exe does the same jobs.

use crate::api;
use crate::yt::{channel_name, tidy, Song};
use std::io::{BufRead, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;

/// One setup at a time (two downloads starting together mustn't both unpack Python).
static SETUP: Mutex<()> = Mutex::new(());

/// The app's library folder (where Android unpacked its lib*.so): the folder of this very library, read from what the
/// process has loaded.
fn native_dir() -> Option<PathBuf> {
    let maps = std::fs::read_to_string("/proc/self/maps").ok()?;
    maps.lines().filter_map(|l| l.split_whitespace().nth(5)).find(|p| p.ends_with("/libbackline_phone.so")).and_then(|p| Path::new(p).parent().map(Path::to_path_buf))
}

fn python_dir() -> PathBuf {
    api::config_dir().join("python")
}

fn ytdlp_dir() -> PathBuf {
    api::config_dir().join("yt-dlp")
}

/// Python unpacked and yt-dlp in place (the first time: a few seconds). Returns what to run: (python, yt-dlp, the
/// library folder).
#[cfg(target_os = "android")]
fn ready() -> Result<(PathBuf, PathBuf, PathBuf), String> {
    let _g = SETUP.lock().unwrap_or_else(|e| e.into_inner());
    let lib = native_dir().ok_or("the app's library folder wasn't found")?;
    let python = lib.join("libpython.so");
    let zip = lib.join("libpython.zip.so");
    if !python.exists() || !zip.exists() {
        return Err("this copy of the app has no yt-dlp in it".to_string());
    }
    // Python's files, unpacked again whenever the app brings a different zip (its size is the version)
    let dir = python_dir();
    let mark = dir.join(".unpacked");
    let size = std::fs::metadata(&zip).map(|m| m.len()).unwrap_or(0).to_string();
    if std::fs::read_to_string(&mark).ok().as_deref() != Some(size.as_str()) {
        let _ = std::fs::remove_dir_all(&dir);
        unzip(&zip, &dir)?;
        std::fs::write(&mark, &size).map_err(|e| e.to_string())?;
    }
    // yt-dlp: the app's own copy the first time, then its updates
    let ytdlp = ytdlp_dir().join("yt-dlp");
    let packed = lib.join("libytdlp.so");
    let packed_size = std::fs::metadata(&packed).map(|m| m.len()).unwrap_or(0);
    let have = std::fs::metadata(&ytdlp).map(|m| m.len()).unwrap_or(0);
    // (a newer app brings a newer yt-dlp: it replaces an older copy that never updated itself)
    let brought = ytdlp_dir().join(".brought");
    if have == 0 || std::fs::read_to_string(&brought).ok().as_deref() != Some(packed_size.to_string().as_str()) {
        std::fs::create_dir_all(ytdlp_dir()).map_err(|e| e.to_string())?;
        std::fs::copy(&packed, &ytdlp).map_err(|e| format!("couldn't set up yt-dlp: {}", e))?;
        let _ = std::fs::write(&brought, packed_size.to_string());
    }
    Ok((python, ytdlp, lib))
}

/// A zip unpacked into `to`, keeping its links and its files' permissions.
#[cfg(target_os = "android")]
fn unzip(zip: &Path, to: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let f = std::fs::File::open(zip).map_err(|e| e.to_string())?;
    let mut z = zip::ZipArchive::new(std::io::BufReader::new(f)).map_err(|e| e.to_string())?;
    for i in 0..z.len() {
        let mut e = z.by_index(i).map_err(|e| e.to_string())?;
        let Some(rel) = e.enclosed_name() else { continue };
        let out = to.join(rel);
        if e.is_dir() {
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(p) = out.parent() {
            std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        let mode = e.unix_mode().unwrap_or(0o644);
        if mode & 0o170000 == 0o120000 {
            let mut target = String::new();
            e.read_to_string(&mut target).map_err(|e| e.to_string())?;
            let _ = std::fs::remove_file(&out);
            std::os::unix::fs::symlink(target.trim(), &out).map_err(|e| e.to_string())?;
            continue;
        }
        let mut w = std::fs::File::create(&out).map_err(|e| e.to_string())?;
        std::io::copy(&mut e, &mut w).map_err(|e| e.to_string())?;
        let _ = std::fs::set_permissions(&out, std::fs::Permissions::from_mode(mode & 0o777 | 0o600));
    }
    Ok(())
}

/// yt-dlp, ready to be given its arguments.
fn command() -> Result<Command, String> {
    #[cfg(target_os = "android")]
    {
        let (python, ytdlp, lib) = ready()?;
        let py = python_dir().join("usr");
        let tmp = api::config_dir().join("tmp");
        let _ = std::fs::create_dir_all(&tmp);
        let mut c = Command::new(python);
        c.arg(ytdlp)
            .env("LD_LIBRARY_PATH", py.join("lib"))
            .env("SSL_CERT_FILE", py.join("etc/tls/cert.pem"))
            .env("PYTHONHOME", &py)
            .env("HOME", &py)
            .env("TMPDIR", &tmp)
            .env("PATH", format!("{}:{}", std::env::var("PATH").unwrap_or_default(), lib.display()))
            // QuickJS for YouTube's challenges, and yt-dlp's cache (the player code it worked out) kept between runs
            .arg("--js-runtimes")
            .arg(format!("quickjs:{}", lib.join("libqjs.so").display()))
            .arg("--cache-dir")
            .arg(ytdlp_dir().join("cache"));
        return Ok(c);
    }
    #[allow(unreachable_code)]
    {
        // (the PC copy: the desktop's own yt-dlp)
        let exe = dirs::config_dir().unwrap_or_default().join("backline").join("bin").join("yt-dlp.exe");
        if !exe.exists() {
            return Err("no yt-dlp on this computer (the desktop app downloads it)".to_string());
        }
        let mut c = Command::new(exe);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            c.creation_flags(0x0800_0000);
        }
        Ok(c)
    }
}

/// A yt-dlp run to the end: its output, each line also handed to `line` as it comes (progress). An error gives the
/// last thing yt-dlp said.
fn run(args: &[&str], mut line: impl FnMut(&str)) -> Result<String, String> {
    let mut c = command()?;
    c.args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let t0 = std::time::Instant::now();
    let mut child = c.spawn().map_err(|e| format!("yt-dlp didn't start: {}", e))?;
    let err = child.stderr.take();
    let err_thread = std::thread::spawn(move || {
        let mut s = String::new();
        if let Some(mut e) = err {
            let _ = e.read_to_string(&mut s);
        }
        s
    });
    let mut out = String::new();
    if let Some(o) = child.stdout.take() {
        for l in std::io::BufReader::new(o).lines().map_while(Result::ok) {
            line(&l);
            out.push_str(&l);
            out.push('\n');
        }
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    let err = err_thread.join().unwrap_or_default();
    api::log(&format!("yt-dlp {} -> {} in {:.1} s", args.first().copied().unwrap_or(""), status.code().unwrap_or(-1), t0.elapsed().as_secs_f32()));
    if status.success() {
        return Ok(out);
    }
    for l in err.lines().rev().take(6) {
        api::log(&format!("  {}", l));
    }
    // the last error line, without yt-dlp's prefixes
    let last = err.lines().rev().find(|l| l.contains("ERROR")).or_else(|| err.lines().rev().find(|l| !l.trim().is_empty())).unwrap_or("yt-dlp failed");
    let last = last.trim().trim_start_matches("ERROR:").trim();
    let last = last.split_once("] ").filter(|(a, _)| a.starts_with('[')).map(|(_, b)| b.trim()).unwrap_or(last);
    Err(last.chars().take(160).collect())
}

/// The song's name and who made it, as the desktop takes them from yt-dlp: YouTube Music's own fields when the video
/// has them, else the video's title and channel.
fn song_from(v: &serde_json::Value, vid: &str) -> Song {
    let s = |k: &str| v[k].as_str().map(str::to_string).filter(|t| !t.trim().is_empty());
    let channel = s("artist").or_else(|| s("channel")).or_else(|| s("uploader")).unwrap_or_default();
    let title = match (s("track"), s("artist")) {
        (Some(t), Some(a)) => format!("{} - {}", a.split(',').next().unwrap_or(&a).trim(), t),
        _ => s("title").unwrap_or_else(|| vid.to_string()),
    };
    Song { vid: vid.to_string(), title: tidy(&title), channel: channel_name(&channel), secs: v["duration"].as_f64().unwrap_or(0.0) as f32, ..Default::default() }
}

/// A link's video: its name, channel and length.
pub(crate) fn details(vid: &str) -> Result<Song, String> {
    let url = format!("https://www.youtube.com/watch?v={}", vid);
    let out = run(&["-j", "--no-playlist", "--skip-download", "--no-warnings", &url], |_| {})?;
    let v: serde_json::Value = serde_json::from_str(out.lines().last().unwrap_or("")).map_err(|_| "yt-dlp's answer couldn't be read".to_string())?;
    if v["is_live"].as_bool() == Some(true) {
        return Err("it's a live stream".to_string());
    }
    Ok(song_from(&v, vid))
}

/// YouTube's videos for `q` (when the quick search doesn't work).
pub(crate) fn search(q: &str) -> Result<Vec<Song>, String> {
    let what = format!("ytsearch15:{}", q);
    let out = run(&["-j", "--flat-playlist", "--no-warnings", &what], |_| {})?;
    Ok(out
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter(|v| v["live_status"].as_str().is_none_or(|s| s == "not_live" || s == "was_live"))
        .filter_map(|v| {
            let id = v["id"].as_str()?.to_string();
            (id.len() == 11).then(|| song_from(&v, &id))
        })
        .collect())
}

/// A video's audio into `out` (AAC in an m4a, which the player reads): `pct` follows the download (0-100).
pub(crate) fn audio(vid: &str, out: &Path, pct: &std::sync::atomic::AtomicU32) -> Result<(), String> {
    let url = format!("https://www.youtube.com/watch?v={}", vid);
    let part = out.with_extension("download");
    let _ = std::fs::remove_file(&part);
    if let Some(d) = out.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let part_s = part.to_string_lossy().to_string();
    let args = ["-f", "140/139/bestaudio[ext=m4a]/bestaudio[acodec^=mp4a]", "--no-playlist", "--no-part", "--no-mtime", "--newline", "--no-warnings", "-o", &part_s, &url];
    let got = run(&args, |l| {
        // [download]  45.3% of    7.96MiB at ...
        if let Some(p) = l.strip_prefix("[download]").and_then(|r| r.split_whitespace().next()).and_then(|t| t.strip_suffix('%')).and_then(|t| t.parse::<f32>().ok()) {
            pct.store(p.clamp(0.0, 100.0) as u32, std::sync::atomic::Ordering::Relaxed);
        }
    });
    match got {
        Ok(_) if std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0) > 0 => std::fs::rename(&part, out).map_err(|e| e.to_string()),
        Ok(_) => Err("the download came back empty".to_string()),
        Err(e) => {
            let _ = std::fs::remove_file(&part);
            Err(e)
        }
    }
}

/// yt-dlp's version (none until it has run once).
pub(crate) fn version() -> Result<String, String> {
    run(&["--version"], |_| {}).map(|v| v.trim().to_string())
}

/// yt-dlp brought up to date from its GitHub releases (what it said).
pub(crate) fn update() -> Result<String, String> {
    // (the PC copy borrows the desktop's yt-dlp: the desktop keeps that one up to date)
    if cfg!(not(target_os = "android")) {
        return Ok("The PC copy uses the desktop's yt-dlp".to_string());
    }
    let out = run(&["-U"], |_| {})?;
    let _ = std::fs::write(ytdlp_dir().join(".checked"), "");
    Ok(out.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("").trim().to_string())
}

/// Whether yt-dlp last looked for an update over a day ago.
pub(crate) fn update_due() -> bool {
    std::fs::metadata(ytdlp_dir().join(".checked")).and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).is_none_or(|e| e.as_secs() > 20 * 3600)
}
