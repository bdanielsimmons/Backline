//! The classic Windows Media Player visualizers, as Ryan Circelli's wmp-legacy-visualizers (MIT) ports them. Their
//! engine runs in retro.dll (the repo's retro/ folder: V8 and the bundled script), which is downloaded from
//! Backline's site the first time one is picked, as the stem tool's runtime is. A thread of its own feeds it the
//! player's samples and draws at the chosen frame rate; the visuals' slots show its newest frame. Windows only for now.

use crate::player::Viz;
use eframe::egui::{self, Color32, Pos2, Rect};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

/// The library this build wants (its retro_version): a new one is a new file on the site and on disk.
const VERSION: u32 = 1;

/// A preset: its family's key (the engine's name for it), its number in the family, the family and its own name.
pub(crate) struct Entry {
    pub kind: &'static str,
    pub preset: i32,
    pub group: &'static str,
    pub name: &'static str,
}

/// Every preset (the library's own list, made with it: retro/presets.txt), the families A to Z, each one's presets in
/// WMP's order. None off Windows.
pub(crate) fn list() -> &'static [Entry] {
    static L: OnceLock<Vec<Entry>> = OnceLock::new();
    L.get_or_init(|| {
        if !cfg!(windows) {
            return Vec::new();
        }
        let mut l: Vec<Entry> = include_str!("../../../retro/presets.txt")
            .lines()
            .filter_map(|l| {
                let mut f = l.split('\t');
                Some(Entry { kind: f.next()?, preset: f.next()?.parse().ok()?, group: f.next()?, name: f.next()? })
            })
            .collect();
        l.sort_by_key(|e| e.group.to_lowercase());
        l
    })
}

/// The families, in order.
pub(crate) fn families() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = Vec::new();
    for e in list() {
        if v.last() != Some(&e.group) {
            v.push(e.group);
        }
    }
    v
}

/// Frames drawn a second: 0 is WMP's own pace (up to 30, slower when a frame takes long).
pub(crate) static FPS: AtomicU8 = AtomicU8::new(60);
pub(crate) const RATES: [(u8, &str); 7] =
    [(0, "As WMP (up to 30)"), (30, "30 fps"), (45, "45 fps"), (60, "60 fps"), (75, "75 fps"), (90, "90 fps"), (120, "120 fps")];

#[derive(Default)]
struct Frame {
    size: [usize; 2],
    px: Vec<Color32>,
    seq: u64,
    smooth: bool,
}

#[derive(Default)]
struct Shared {
    /// the preset asked for, the view's size in pixels, and when it was last asked for (no one looking: no frames)
    want: Mutex<Option<(usize, [u32; 2], Instant)>>,
    frame: Mutex<Frame>,
    /// what to say over the picture ("DOWNLOADING VISUALS 40%"); empty while it draws
    status: Mutex<String>,
    /// Picture Viz's photo: the playing song's cover
    cover: Mutex<String>,
    running: AtomicBool,
    failed_at: Mutex<Option<Instant>>,
}

fn shared() -> &'static Shared {
    static S: OnceLock<Shared> = OnceLock::new();
    S.get_or_init(Shared::default)
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// The song's cover (a web address), for Picture Viz.
pub(crate) fn set_cover(url: &str) {
    let mut c = lock(&shared().cover);
    if *c != url {
        *c = url.to_string();
    }
}

/// The visuals thread, started the first time one of these is shown (and again a while after it gave up).
pub(crate) fn start(viz: &Arc<Mutex<Viz>>) {
    let sh = shared();
    if !cfg!(windows) || sh.running.load(Ordering::Relaxed) || lock(&sh.failed_at).is_some_and(|t| t.elapsed() < Duration::from_secs(30)) {
        return;
    }
    sh.running.store(true, Ordering::Relaxed);
    let viz = viz.clone();
    let ok = std::thread::Builder::new().name("retro visuals".into()).spawn(move || {
        #[cfg(windows)]
        win::run(sh, &viz);
        let _ = viz;
        *lock(&sh.failed_at) = Some(Instant::now());
        sh.running.store(false, Ordering::Relaxed);
    });
    if ok.is_err() {
        sh.running.store(false, Ordering::Relaxed);
    }
}

/// Preset `entry` in `r`: the newest frame, stretched as WMP stretched it (sharp or smooth, by the preset).
pub(crate) fn paint(p: &egui::Painter, r: Rect, entry: usize) {
    let sh = shared();
    let ppp = p.ctx().pixels_per_point();
    let mut size = [(r.width() * ppp).round().max(16.0) as u32, (r.height() * ppp).round().max(16.0) as u32];
    {
        // (shown in two places at once, the pop-out and the deck say: drawn for the bigger, not resized to and fro)
        let mut w = lock(&sh.want);
        if let Some((e, s, at)) = *w {
            if e == entry && at.elapsed() < Duration::from_millis(100) && s[0] * s[1] > size[0] * size[1] {
                size = s;
            }
        }
        *w = Some((entry, size, Instant::now()));
    }
    p.rect_filled(r, 0.0, Color32::BLACK);
    // each window its own texture (the pop-out window draws on a thread of its own)
    thread_local! {
        static TEX: std::cell::RefCell<Option<(egui::TextureHandle, u64)>> = const { std::cell::RefCell::new(None) };
    }
    let id = TEX.with(|t| {
        let mut t = t.borrow_mut();
        let f = lock(&sh.frame);
        if f.seq == 0 {
            return None;
        }
        let opts = if f.smooth { egui::TextureOptions::LINEAR } else { egui::TextureOptions::NEAREST };
        match t.as_mut() {
            Some((tex, seq)) if *seq == f.seq => Some(tex.id()),
            _ => {
                let img = egui::ColorImage { size: f.size, pixels: f.px.clone() };
                let seq = f.seq;
                drop(f);
                match t.as_mut() {
                    Some((tex, s)) => {
                        tex.set(img, opts);
                        *s = seq;
                    }
                    None => *t = Some((p.ctx().load_texture("retro visuals", img, opts), seq)),
                }
                t.as_ref().map(|(tex, _)| tex.id())
            }
        }
    });
    if let Some(id) = id {
        p.image(id, r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
    }
    let status = lock(&sh.status).clone();
    if !status.is_empty() {
        crate::ptext(p, r.center(), egui::Align::Center, &status, 1.4, Color32::from_white_alpha(200));
    }
}

/// WMP's pacing (the same in every version, as Ryan's player has it): 30 ms between frames until 8 frame times are
/// known, then clamp(max(avg / 0.6, 33.3) - avg, 10, 500) ms after each frame, avg the last 8 frames' time.
fn wmp_delay(hist: &mut Vec<f64>, ms: f64) -> f64 {
    let had = hist.len();
    hist.push(ms);
    if hist.len() > 8 {
        hist.remove(0);
    }
    if had < 8 {
        return 30.0;
    }
    let avg = hist.iter().sum::<f64>() / 8.0;
    ((avg / 0.6).max(100.0 / 3.0) - avg).clamp(10.0, 500.0).trunc()
}

#[cfg(windows)]
mod win {
    use super::{list, lock, wmp_delay, Shared, FPS, VERSION};
    use crate::player::Viz;
    use eframe::egui::Color32;
    use std::io::Read;
    use std::os::windows::ffi::OsStrExt;
    use std::path::PathBuf;
    use std::sync::atomic::Ordering;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};
    use windows::core::{PCSTR, PCWSTR};
    use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

    type H = *mut std::ffi::c_void;

    /// retro.dll's interface (retro/src/lib.rs).
    struct Lib {
        version: unsafe extern "C" fn() -> u32,
        new: unsafe extern "C" fn() -> H,
        free: unsafe extern "C" fn(H),
        open: unsafe extern "C" fn(H, *const u8, usize, i32, i32, i32) -> i32,
        preset: unsafe extern "C" fn(H, i32) -> i32,
        size: unsafe extern "C" fn(H, i32, i32) -> i32,
        rate: unsafe extern "C" fn(H, i32) -> i32,
        push: unsafe extern "C" fn(H, *const f32, usize) -> i32,
        render: unsafe extern "C" fn(H, i32) -> u32,
        pixels: unsafe extern "C" fn(H) -> *const u32,
        smooth: unsafe extern "C" fn(H) -> i32,
        picture: unsafe extern "C" fn(H, *const u8, i32, i32) -> i32,
        error: unsafe extern "C" fn(H, *mut u8, usize) -> usize,
    }

    impl Lib {
        /// The library at `path` (it stays loaded: its engine can't be started twice in one run).
        fn load(path: &std::path::Path) -> Result<Lib, String> {
            let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
            unsafe {
                let m = LoadLibraryW(PCWSTR(wide.as_ptr())).map_err(|e| format!("{} ({})", e.message(), path.display()))?;
                macro_rules! f {
                    ($n:literal) => {
                        std::mem::transmute(GetProcAddress(m, PCSTR(concat!($n, "\0").as_ptr())).ok_or(concat!("it has no ", $n))?)
                    };
                }
                Ok(Lib {
                    version: f!("retro_version"),
                    new: f!("retro_new"),
                    free: f!("retro_free"),
                    open: f!("retro_open"),
                    preset: f!("retro_preset"),
                    size: f!("retro_size"),
                    rate: f!("retro_rate"),
                    push: f!("retro_push"),
                    render: f!("retro_render"),
                    pixels: f!("retro_pixels"),
                    smooth: f!("retro_smooth"),
                    picture: f!("retro_picture"),
                    error: f!("retro_error"),
                })
            }
        }

        fn error(&self, h: H) -> String {
            let mut buf = vec![0u8; 2048];
            let n = unsafe { (self.error)(h, buf.as_mut_ptr(), buf.len()) };
            buf.truncate(n.min(buf.len()));
            String::from_utf8_lossy(&buf).into_owned()
        }
    }

    fn dir() -> PathBuf {
        crate::api::config_dir().join("tools")
    }

    /// Where the library is: BACKLINE_RETRO_DLL (a test copy's own build), else the downloaded one.
    fn dll_path() -> PathBuf {
        match std::env::var_os("BACKLINE_RETRO_DLL") {
            Some(p) => PathBuf::from(p),
            None => dir().join(format!("retro-{}.dll", VERSION)),
        }
    }

    /// The library from Backline's site (zipped, with its license), once; older versions are thrown away.
    fn download(say: &dyn Fn(String)) -> Result<(), String> {
        // (test switch BACKLINE_RETRO_URL: from there instead)
        let url = std::env::var("BACKLINE_RETRO_URL")
            .unwrap_or_else(|_| format!("{}/tools/retro-{}/retro.zip", crate::update::SITE, VERSION));
        let client = reqwest::blocking::Client::builder().timeout(Duration::from_secs(900)).build().map_err(|e| e.to_string())?;
        let mut resp = client.get(&url).send().map_err(|e| e.to_string())?.error_for_status().map_err(|e| e.to_string())?;
        let total = resp.content_length().unwrap_or(0);
        let mut data = Vec::with_capacity(total as usize);
        let mut buf = vec![0u8; 256 * 1024];
        loop {
            let n = resp.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            data.extend_from_slice(&buf[..n]);
            if total > 0 {
                say(format!("DOWNLOADING VISUALS {}%", data.len() as u64 * 100 / total));
            }
        }
        say("UNPACKING VISUALS".to_string());
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(data)).map_err(|e| e.to_string())?;
        let mut dll = Vec::new();
        zip.by_name("retro.dll").map_err(|e| e.to_string())?.read_to_end(&mut dll).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(dir()).map_err(|e| e.to_string())?;
        let mut lic = Vec::new();
        if let Ok(mut f) = zip.by_name("LICENSE-wmp-legacy-visualizers.txt") {
            if f.read_to_end(&mut lic).is_ok() {
                let _ = std::fs::write(dir().join("LICENSE-wmp-legacy-visualizers.txt"), lic);
            }
        }
        let part = dll_path().with_extension("part");
        std::fs::write(&part, dll).map_err(|e| e.to_string())?;
        std::fs::rename(&part, dll_path()).map_err(|e| e.to_string())?;
        if let Ok(rd) = std::fs::read_dir(dir()) {
            let keep = dll_path();
            for e in rd.flatten() {
                let n = e.file_name().to_string_lossy().to_string();
                if n.starts_with("retro-") && n.ends_with(".dll") && e.path() != keep {
                    let _ = std::fs::remove_file(e.path());
                }
            }
        }
        Ok(())
    }

    /// Picture Viz's photo: the cover at `url`, as RGBA.
    fn fetch_cover(url: &str) -> Option<(Vec<u8>, i32, i32)> {
        if !url.starts_with("http") {
            return None;
        }
        let bytes = reqwest::blocking::Client::builder().timeout(Duration::from_secs(8)).build().ok()?.get(url).send().ok()?.bytes().ok()?;
        let img = image::load_from_memory(&bytes).ok()?.to_rgba8();
        let (w, h) = (img.width() as i32, img.height() as i32);
        Some((img.into_raw(), w, h))
    }

    pub(super) fn run(sh: &'static Shared, viz: &Mutex<Viz>) {
        let say = |s: String| *lock(&sh.status) = s;
        if !dll_path().exists() {
            say("DOWNLOADING VISUALS".to_string());
            if let Err(e) = download(&say) {
                crate::api::log(&format!("retro visuals: download failed: {}", e));
                say("COULDN'T DOWNLOAD THE VISUALS (TRY AGAIN LATER)".to_string());
                return;
            }
        }
        say("STARTING VISUALS".to_string());
        let lib = match Lib::load(&dll_path()) {
            Ok(l) if unsafe { (l.version)() } == VERSION => l,
            Ok(_) => {
                say("THE VISUALS NEED AN UPDATE".to_string());
                return;
            }
            Err(e) => {
                crate::api::log(&format!("retro visuals: couldn't load the library: {}", e));
                say("THE VISUALS COULDN'T START".to_string());
                return;
            }
        };
        let mut h: H = std::ptr::null_mut();
        let mut cur: Option<usize> = None;
        let mut cur_kind = "";
        let mut cur_size = [0u32; 2];
        let mut cur_cover = String::new();
        let mut rate = 0u32;
        let mut seen_total = 0u64;
        let mut heard_at = Instant::now();
        let mut pcm: Vec<f32> = Vec::new();
        let mut due = Instant::now();
        let mut hist: Vec<f64> = Vec::new();
        let mut idle_since = Instant::now();
        let mut px: Vec<Color32> = Vec::new();
        // (test switch BACKLINE_RETRO_STATS: frames, their time and the loop's turns every 5 s, in the log)
        let stats = std::env::var_os("BACKLINE_RETRO_STATS").is_some();
        let (mut st_at, mut st_n, mut st_ms, mut st_turns) = (Instant::now(), 0u32, 0f64, 0u32);
        loop {
            st_turns += 1;
            if stats && st_at.elapsed() > Duration::from_secs(5) {
                let s = st_at.elapsed().as_secs_f64();
                crate::api::log(&format!(
                    "retro stats: {} {}x{}: {:.1} frames/s, {:.2} ms each, {:.0} turns/s",
                    cur_kind, cur_size[0], cur_size[1], st_n as f64 / s, st_ms / st_n.max(1) as f64, st_turns as f64 / s
                ));
                (st_at, st_n, st_ms, st_turns) = (Instant::now(), 0, 0.0, 0);
            }
            let want = *lock(&sh.want);
            let looked = want.filter(|w| w.2.elapsed() < Duration::from_millis(1500));
            let Some((entry, size, _)) = looked else {
                // no one has looked for a while: nothing drawn, and after two minutes the engine is let go
                if !h.is_null() && idle_since.elapsed() > Duration::from_secs(120) {
                    unsafe { (lib.free)(h) };
                    h = std::ptr::null_mut();
                    cur = None;
                }
                std::thread::sleep(Duration::from_millis(60));
                continue;
            };
            idle_since = Instant::now();
            let Some(e) = list().get(entry) else {
                std::thread::sleep(Duration::from_millis(60));
                continue;
            };
            if h.is_null() {
                h = unsafe { (lib.new)() };
                if h.is_null() {
                    crate::api::log(&format!("retro visuals: the engine didn't start: {}", lib.error(h)));
                    say("THE VISUALS COULDN'T START".to_string());
                    return;
                }
                say(String::new());
                cur = None;
                cur_cover.clear();
            }
            // the song's cover, for Picture Viz (fetched here, off the screen's thread)
            let cover = lock(&sh.cover).clone();
            if cover != cur_cover {
                cur_cover = cover.clone();
                let (rgba, w, hh) = fetch_cover(&cover).unwrap_or((Vec::new(), 0, 0));
                unsafe { (lib.picture)(h, rgba.as_ptr(), w, hh) };
            }
            // another preset: the same family keeps its state (as WMP's), another starts afresh
            if cur != Some(entry) {
                let ok = if cur.is_some() && cur_kind == e.kind {
                    unsafe { (lib.preset)(h, e.preset) }
                } else {
                    cur_size = size;
                    unsafe { (lib.open)(h, e.kind.as_ptr(), e.kind.len(), e.preset, size[0] as i32, size[1] as i32) }
                };
                if ok == 0 {
                    crate::api::log(&format!("retro visuals: {} {} didn't start: {}", e.kind, e.preset, lib.error(h)));
                }
                cur = Some(entry);
                cur_kind = e.kind;
                lock(&sh.frame).smooth = unsafe { (lib.smooth)(h) } != 0;
                hist.clear();
                due = Instant::now();
            }
            if size != cur_size {
                cur_size = size;
                unsafe { (lib.size)(h, size[0] as i32, size[1] as i32) };
            }
            // the sound since last time: both channels, interleaved
            {
                let v = lock(viz);
                if v.rate != rate && v.rate > 0 {
                    rate = v.rate;
                    unsafe { (lib.rate)(h, rate as i32) };
                }
                let n = (v.total.saturating_sub(seen_total) as usize).min(v.samples.len()).min(v.first.len());
                seen_total = v.total;
                pcm.clear();
                let (l, r) = (&v.first[v.first.len() - n..], &v.samples[v.samples.len() - n..]);
                for i in 0..n {
                    pcm.push(l[i]);
                    pcm.push(r[i]);
                }
            }
            if !pcm.is_empty() {
                heard_at = Instant::now();
                unsafe { (lib.push)(h, pcm.as_ptr(), pcm.len()) };
            }
            let now = Instant::now();
            if now < due {
                std::thread::sleep((due - now).min(Duration::from_millis(4)));
                continue;
            }
            // nothing heard for a moment: paused (WMP's state 1), and the picture holds (Particle's spin goes on)
            let paused = heard_at.elapsed() > Duration::from_millis(300);
            let fps = FPS.load(Ordering::Relaxed);
            if paused && cur_kind != "particle" && lock(&sh.frame).seq > 0 {
                due = now + Duration::from_millis(30);
                continue;
            }
            let t0 = Instant::now();
            let wh = unsafe { (lib.render)(h, if paused { 1 } else { 2 }) };
            let ms = t0.elapsed().as_secs_f64() * 1000.0;
            st_n += 1;
            st_ms += ms;
            let (w, hh) = ((wh & 0xffff) as usize, (wh >> 16) as usize);
            let src = unsafe { (lib.pixels)(h) };
            if w > 0 && hh > 0 && !src.is_null() {
                let s = unsafe { std::slice::from_raw_parts(src, w * hh) };
                px.clear();
                px.extend(s.iter().map(|&p| Color32::from_rgb((p >> 16) as u8, (p >> 8) as u8, p as u8)));
                let mut f = lock(&sh.frame);
                std::mem::swap(&mut f.px, &mut px);
                f.size = [w, hh];
                f.seq += 1;
            }
            // the next frame: at WMP's pace, or the rate picked (a late one starts afresh rather than catching up)
            due = if fps == 0 {
                let base = if now.duration_since(due) < Duration::from_millis(33) { due } else { now };
                base + Duration::from_secs_f64((ms + wmp_delay(&mut hist, ms)) / 1000.0)
            } else {
                let next = due + Duration::from_secs_f64(1.0 / fps as f64);
                if next < now {
                    now
                } else {
                    next
                }
            };
        }
    }
}
