//! Self-update. Every release is build `v9.N` (N = the Actions run number, which the build stamps into the program
//! as BACKLINE_BUILD). Releases are put on Backline's own download site (`SITE`, on GitHub Pages), where
//! `latest.json` names the newest build of each file with its SHA-256, and `notes.json` holds the latest releases'
//! notes; GitHub's releases are asked only when the site can't be (or isn't set up yet). The app downloads the
//! file for this system next to itself, checks it against its SHA-256, and swaps it in on the next start (or
//! right away on click). The previous file is kept as `.old` for one start so a bad swap can be undone by hand.

use crate::{api, retro_btn, Action, App, Msg, Tip};
use eframe::egui;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const REPO: &str = "bdanielsimmons/Backline";

/// Backline's download site (no slash at the end): GitHub Pages, made by the release workflow's `site` job. It stays
/// public when the repository is private (that needs GitHub Pro on the account).
const SITE: &str = "https://bdanielsimmons.github.io/Backline";

#[derive(Clone)]
pub struct Info {
    pub build: u64,
    pub tag: String,
    pub url: String,
    /// the file's SHA-256 (lowercase hex) when the site gave one: a download that doesn't match is thrown away
    pub sha256: Option<String>,
}

fn es<E: ToString>(e: E) -> String {
    e.to_string()
}

/// This program's build number (0 for a local build that was not made by the release workflow).
pub fn build() -> u64 {
    option_env!("BACKLINE_BUILD").and_then(|s| s.parse().ok()).unwrap_or(0)
}

/// Name of the release file that matches this program. One app for everyone (SIMPLE is a switch now), so a copy of
/// the old simple edition takes this one too and keeps its switch on.
pub(crate) fn asset_name() -> &'static str {
    if cfg!(windows) {
        "backline-practice.exe"
    } else {
        "backline-practice-mac.tar.gz"
    }
}

fn http() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder().timeout(Duration::from_secs(600)).user_agent("Backline-updater").build().map_err(es)
}

fn staged_path() -> Result<PathBuf, String> {
    Ok(std::env::current_exe().map_err(es)?.with_extension("new"))
}

fn marker() -> PathBuf {
    api::config_dir().join("update.txt")
}

/// GitHub's ETag for the last "latest release" answer. Sending it back makes an unchanged check a 304 reply,
/// which is tiny and does not count against GitHub's rate limit.
static ETAG: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// Ask for the newest release; Some when it is newer than this build. The site first, GitHub if it can't be read.
pub fn check() -> Result<Option<Info>, String> {
    let me = build();
    if me == 0 {
        return Ok(None);
    }
    if !SITE.is_empty() {
        match site_json("latest.json") {
            Ok((site, v)) => return Ok(pick(&v, &site, asset_name(), me)),
            Err(e) => api::log(&format!("update: the download site can't be read ({}), asking GitHub", e)),
        }
    }
    check_github(me)
}

/// One of the site's files, read as JSON, and the site it came from. `latest.json` can say the site has moved
/// (`moved_to`): the new one is asked instead (once), so the address can change without stranding older copies.
fn site_json(name: &str) -> Result<(String, serde_json::Value), String> {
    let get = |site: &str, name: &str| -> Result<serde_json::Value, String> {
        http()?
            .get(format!("{}/{}", site, name))
            .timeout(Duration::from_secs(20))
            .send()
            .map_err(es)?
            .error_for_status()
            .map_err(es)?
            .json()
            .map_err(es)
    };
    let latest = get(SITE, "latest.json")?;
    let site = match latest["moved_to"].as_str().map(|s| s.trim_end_matches('/')) {
        Some(m) if m.starts_with("https://") && m != SITE => m.to_string(),
        _ => SITE.to_string(),
    };
    if name == "latest.json" && site == SITE {
        return Ok((site, latest));
    }
    Ok((site.clone(), get(&site, name)?))
}

/// From the site's `latest.json`: this system's file when its build is newer than `me`.
fn pick(v: &serde_json::Value, site: &str, asset: &str, me: u64) -> Option<Info> {
    let f = &v["files"][asset];
    let b = f["build"].as_u64()?;
    let path = f["path"].as_str()?;
    let sha = f["sha256"].as_str().filter(|h| h.len() == 64)?.to_ascii_lowercase();
    (b > me).then(|| Info { build: b, tag: format!("v9.{}", b), url: format!("{}/{}", site, path), sha256: Some(sha) })
}

/// GitHub's latest release, from before the site (and while the repository is public).
fn check_github(me: u64) -> Result<Option<Info>, String> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", REPO);
    let mut req = http()?.get(url).header("Accept", "application/vnd.github+json").timeout(Duration::from_secs(20));
    if let Some(t) = ETAG.lock().ok().and_then(|g| g.clone()) {
        req = req.header("If-None-Match", t);
    }
    let resp = req.send().map_err(es)?;
    if resp.status().as_u16() == 304 {
        return Ok(None);
    }
    let resp = resp.error_for_status().map_err(es)?;
    if let Some(t) = resp.headers().get("etag").and_then(|h| h.to_str().ok()) {
        if let Ok(mut g) = ETAG.lock() {
            *g = Some(t.to_string());
        }
    }
    let v: serde_json::Value = resp.json().map_err(es)?;
    let tag = v["tag_name"].as_str().ok_or("release has no tag")?.to_string();
    let Some(n) = tag.rsplit('.').next().and_then(|s| s.parse::<u64>().ok()) else { return Ok(None) };
    if n <= me {
        return Ok(None);
    }
    let want = asset_name();
    let url = v["assets"]
        .as_array()
        .and_then(|a| a.iter().find(|x| x["name"].as_str() == Some(want)))
        .and_then(|x| x["browser_download_url"].as_str())
        .ok_or("the release has no file for this system")?
        .to_string();
    Ok(Some(Info { build: n, tag, url, sha256: None }))
}

/// The notes of the latest `n` releases up to this one, newest first: (tag, notes). For the WHAT'S NEW panel.
pub fn notes(n: usize) -> Result<Vec<(String, String)>, String> {
    let me = build();
    if !SITE.is_empty() {
        match site_json("notes.json") {
            Ok((_, v)) => return Ok(notes_from(&v, me, n)),
            Err(e) => api::log(&format!("update: the site's notes can't be read ({}), asking GitHub", e)),
        }
    }
    let url = format!("https://api.github.com/repos/{}/releases?per_page=30", REPO);
    let v: serde_json::Value = http()?
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .timeout(Duration::from_secs(20))
        .send()
        .map_err(es)?
        .error_for_status()
        .map_err(es)?
        .json()
        .map_err(es)?;
    let mut out: Vec<(u64, String, String)> = v
        .as_array()
        .ok_or("no releases")?
        .iter()
        .filter(|r| !r["prerelease"].as_bool().unwrap_or(false))
        .filter_map(|r| {
            let tag = r["tag_name"].as_str()?.to_string();
            let n = tag.rsplit('.').next()?.parse::<u64>().ok()?;
            Some((n, tag, r["body"].as_str().unwrap_or("").to_string()))
        })
        // a local build (0) shows the newest ones
        .filter(|(n, ..)| me == 0 || *n <= me)
        .collect();
    out.sort_by_key(|a| std::cmp::Reverse(a.0));
    Ok(out.into_iter().take(n).map(|(_, t, b)| (t, b)).collect())
}

/// The site's `notes.json` (newest first: build, tag, body): the latest `n` up to build `me` (any, for a local build).
fn notes_from(v: &serde_json::Value, me: u64, n: usize) -> Vec<(String, String)> {
    v.as_array()
        .into_iter()
        .flatten()
        .filter(|r| r["build"].as_u64().is_some_and(|b| me == 0 || b <= me))
        .filter_map(|r| Some((r["tag"].as_str()?.to_string(), r["body"].as_str().unwrap_or("").to_string())))
        .take(n)
        .collect()
}

/// SHA-256 of the bytes, as lowercase hex.
fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(bytes).iter().map(|b| format!("{:02x}", b)).collect()
}

/// The downloaded file, or why it can't be used: the wrong SHA-256 (cut short, or not what the site published).
fn download(info: &Info) -> Result<Vec<u8>, String> {
    let bytes = http()?.get(&info.url).send().map_err(es)?.error_for_status().map_err(es)?.bytes().map_err(es)?.to_vec();
    if let Some(want) = &info.sha256 {
        if sha256_hex(&bytes) != *want {
            return Err("the download doesn't match its checksum, so it wasn't used".to_string());
        }
    }
    Ok(bytes)
}

/// An update's `url` when it comes from the linked computer (SYNC) instead of a release.
pub(crate) const FROM_LINKED: &str = "sync:linked";

/// The linked computer runs a newer Backline of the same kind as this one: that update (taken over the link, no
/// waiting for the release to be built).
pub(crate) fn check_linked() -> Option<Info> {
    let me = build();
    if me == 0 {
        return None;
    }
    let (b, kind) = crate::sync::peer_app().ok()?;
    (b > me && kind == asset_name())
        .then(|| Info { build: b, tag: format!("v9.{}", b), url: FROM_LINKED.to_string(), sha256: None })
}

/// Download the new program and leave it next to this one as `<name>.new`.
pub fn stage(info: &Info) -> Result<(), String> {
    let dest = staged_path()?;
    let _ = std::fs::remove_file(&dest);
    if info.url == FROM_LINKED {
        // the linked computer's own copy, over the link, checked as it comes
        crate::sync::fetch_app(&dest)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755)).map_err(es)?;
        }
    } else if cfg!(windows) {
        let bytes = download(info)?;
        std::fs::write(&dest, &bytes).map_err(es)?;
    } else {
        // the Mac / Linux file is a .tar.gz holding one program called `backline`
        let bytes = download(info)?;
        let dir = api::config_dir().join("update");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).map_err(es)?;
        let tgz = dir.join("new.tar.gz");
        std::fs::write(&tgz, &bytes).map_err(es)?;
        let st = std::process::Command::new("tar").arg("-xzf").arg(&tgz).arg("-C").arg(&dir).status().map_err(es)?;
        if !st.success() {
            return Err("could not unpack the update".to_string());
        }
        std::fs::copy(dir.join("backline"), &dest).map_err(es)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755)).map_err(es)?;
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
    if std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0) < 1_000_000 {
        let _ = std::fs::remove_file(&dest);
        return Err("the downloaded update looks incomplete".to_string());
    }
    std::fs::write(marker(), info.build.to_string()).map_err(es)?;
    Ok(())
}

/// Swap the staged program in and start it. The caller exits afterwards.
pub fn apply() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(es)?;
    let new = staged_path()?;
    let old = exe.with_extension("old");
    if !new.exists() {
        return Err("no update has been downloaded".to_string());
    }
    let _ = std::fs::remove_file(&old);
    // a running program can be renamed (not overwritten) on Windows, and on Mac / Linux too
    std::fs::rename(&exe, &old).map_err(es)?;
    if let Err(e) = std::fs::rename(&new, &exe) {
        let _ = std::fs::rename(&old, &exe);
        return Err(es(e));
    }
    let _ = std::fs::remove_file(marker());
    std::process::Command::new(&exe).spawn().map_err(es)?;
    Ok(())
}

/// Called first thing in main(): finish an update that was downloaded during the last session.
/// True when the new program has been started and this one should just exit.
pub fn apply_staged_at_start() -> bool {
    if let Ok(old) = std::env::current_exe().map(|e| e.with_extension("old")) {
        let _ = std::fs::remove_file(old);
    }
    let Ok(txt) = std::fs::read_to_string(marker()) else { return false };
    if txt.trim().parse::<u64>().unwrap_or(0) <= build() {
        // already running that build (or newer): drop the leftovers
        let _ = std::fs::remove_file(marker());
        if let Ok(p) = staged_path() {
            let _ = std::fs::remove_file(p);
        }
        return false;
    }
    api::log("update at start: installing the downloaded version");
    match apply() {
        Ok(()) => true,
        Err(e) => {
            api::log(&format!("update at start: {}", e));
            false
        }
    }
}

impl App {
    pub(crate) fn start_update_check(&mut self) {
        self.upd_checked = Some(Instant::now());
        let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let _ = tx.send(Msg::UpdateFound(check()));
            ctx.request_repaint();
        });
    }

    /// Checks at start and every 6 hours. A downloaded update shows one bar at the top (click to restart); it also installs on the next launch.
    /// It only restarts by itself if the user turned that on in Preferences (off by default).
    pub(crate) fn update_banner(&mut self, ctx: &egui::Context, acts: &mut Vec<Action>) {
        let due = self.upd_checked.is_none_or(|t| t.elapsed() > Duration::from_secs(6 * 3600));
        if due && matches!(self.upd_state, 0 | 3) {
            self.start_update_check();
        }
        // linked with a computer running a newer Backline: take it from there, as soon as it says so (it says which
        // build it runs every few seconds), and every two minutes in case it's one that doesn't say
        static ASKED_FOR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let there = crate::sync::peer_build();
        let news = there > build() && ASKED_FOR.load(std::sync::atomic::Ordering::Relaxed) != there;
        let ask_linked = news || self.linked_upd_at.is_none_or(|t| t.elapsed() > Duration::from_secs(120));
        if ask_linked && matches!(self.upd_state, 0 | 3) && build() > 0 && crate::sync::peer_name().is_some() {
            ASKED_FOR.store(there, std::sync::atomic::Ordering::Relaxed);
            self.linked_upd_at = Some(Instant::now());
            let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
            std::thread::spawn(move || {
                if let Some(info) = check_linked() {
                    api::log(&format!("update: the linked computer has {}, taking it over the link", info.tag));
                    let _ = tx.send(Msg::UpdateFound(Ok(Some(info))));
                    ctx.request_repaint();
                }
            });
        }
        // the linked computer's logs, kept here (so a crash over there can be looked into from this one)
        crate::sync::keep_peer_logs();
        // any mouse or key activity counts as being here
        if ctx.input(|i| i.pointer.is_moving() || !i.events.is_empty()) {
            self.last_active = Instant::now();
        }
        if self.upd_state != 2 {
            return;
        }
        // restart by itself once nothing is playing and you have been away a while: when it's turned on in
        // Preferences, and always for a version taken from your own linked computer (after a minute, not five)
        let playing = self.cur.is_some() && !self.paused && !self.stopped;
        let from_link = self.upd.as_ref().is_some_and(|i| i.url == FROM_LINKED);
        let away = Duration::from_secs(if from_link { 60 } else { 300 });
        if (self.auto_restart || from_link) && !playing && self.last_active.elapsed() > away {
            api::log("update: nothing playing and no one at it: restarting on the new version");
            acts.push(Action::ApplyUpdate);
            return;
        }
        if self.auto_restart || from_link {
            ctx.request_repaint_after(Duration::from_secs(15));
        }
        let Some(info) = self.upd.clone() else { return };
        // (over everything, Preferences and the other dialogs too, so it can always be clicked)
        egui::Area::new(egui::Id::new("update_banner"))
            .order(egui::Order::Tooltip)
            .anchor(egui::Align2::CENTER_TOP, [0.0, 4.0])
            .show(ctx, |ui| {
                let tip = if from_link {
                    "Saves everything and restarts Backline on the new version from your linked computer. It also restarts by itself once nothing is playing and no one has used it for a minute."
                } else {
                    "Saves everything and restarts Backline on the new version. Otherwise it installs the next time you open the app. You can turn on automatic restart in Preferences."
                };
                if retro_btn(ui, &format!("UPDATE {} READY - CLICK TO RESTART", info.tag.to_uppercase()), true)
                    .tip(tip)
                    .clicked()
                {
                    acts.push(Action::ApplyUpdate);
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_is_the_standard_one() {
        assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

    #[test]
    fn the_site_offers_this_systems_newer_file() {
        let h = "ab".repeat(32);
        let v = serde_json::json!({ "files": {
            "backline-practice.exe": { "build": 380, "path": "builds/380/backline-practice.exe", "sha256": h },
            "backline-practice-mac.tar.gz": { "build": 376, "path": "builds/376/backline-practice-mac.tar.gz", "sha256": h.to_uppercase() },
        }});
        let i = pick(&v, "https://dl.example", "backline-practice.exe", 372).unwrap();
        assert_eq!((i.build, i.tag.as_str(), i.url.as_str()), (380, "v9.380", "https://dl.example/builds/380/backline-practice.exe"));
        assert_eq!(i.sha256.as_deref(), Some(h.as_str()));
        // the Mac file is its own build (built less often); the hash is compared in lowercase
        let mac = pick(&v, "https://dl.example", "backline-practice-mac.tar.gz", 372).unwrap();
        assert_eq!((mac.build, mac.sha256.as_deref()), (376, Some(h.as_str())));
        // not newer, not there, or no proper hash: nothing to take
        assert!(pick(&v, "https://dl.example", "backline-practice.exe", 380).is_none());
        assert!(pick(&v, "https://dl.example", "backline-linux.tar.gz", 1).is_none());
        let bad = serde_json::json!({ "files": { "backline-practice.exe": { "build": 400, "path": "x", "sha256": "abc" } } });
        assert!(pick(&bad, "https://dl.example", "backline-practice.exe", 1).is_none());
    }

    #[test]
    fn notes_stop_at_this_build() {
        let v = serde_json::json!([
            { "build": 382, "tag": "v9.382", "body": "c" },
            { "build": 381, "tag": "v9.381", "body": "b" },
            { "build": 380, "tag": "v9.380", "body": "a" },
        ]);
        let tags = |me, n| notes_from(&v, me, n).into_iter().map(|(t, _)| t).collect::<Vec<_>>();
        assert_eq!(tags(381, 5), ["v9.381", "v9.380"]);
        assert_eq!(tags(0, 2), ["v9.382", "v9.381"]);
    }
}
