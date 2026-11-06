//! SYNC: your computers on the same network keep the same plan, progress, lists and practice, with no account and
//! no cloud. Linked once by typing one computer's code on the other (`link`, `Engine::link_talk`: they share a key),
//! the two Backlines find each other by a broadcast on the local network (`Engine::find`) and swap what changed since
//! they last agreed (`Engine::round`; the computer that's called decides, item by item, in `decide`). Either one may
//! call the other: a computer whose firewall keeps calls out (Windows asks once, on that computer) still syncs by
//! calling, and hears of the other by its answers to its own broadcasts.
//!
//! Only what you made goes across, never songs (`read_here`): the plan's files, the atonal lines and the books (whole
//! files), and library.json part by part (each top-level key on its own, so a diary day on one computer and an ear
//! drill on the other both keep). Each computer keeps its own song files, folders, takes and what it played (`LOCAL`),
//! and the app keeps a song that came over in the sync only when it's played there. An item changed on one side since
//! the two last agreed: that side's goes across. Changed on both: lists, logs and maps are put together (`merge`), and
//! the plan and the books go to the newer change. The first time, the computer where the code was typed takes the
//! other's things as they are (`fresh`: nothing of its own goes across). What gets replaced is kept in sync_backups
//! first.
//!
//! Two versions of Backline may write a part a little differently (one knows a field the other doesn't): each side
//! keeps what the two agreed on in its own form (`Peer::base`), so a part neither changed is left alone, a part that
//! came in counts as this side saved it (`took_lib`), and fields only this side knows are kept in what comes in
//! (`keep_ours`: a newer Backline's diary notes survive an older one's changes to the day).
//!
//! Changes are looked for once a second (`Engine::poll`). A file that comes in is written whole (tmp, then renamed)
//! and the app is told to read it again (`Incoming::Files`); library parts go to the app, which puts them in its
//! library and saves it (`Incoming::Library`, then `library_saved`; library.json isn't read till then). sync.json (on
//! or off, this computer's id and code, the linked computer, its key, the last agreed hashes) never goes across.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::hash::Hasher;
use std::io::{ErrorKind, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// The port Backlines announce themselves on, and the one they sync on (another when that's taken).
const FIND_PORT: u16 = 47390;
const SYNC_PORT: u16 = 47391;
/// Whole files that sync, in Backline's folder.
const FILES: [&str; 7] = [
    "plan.json",
    "plan_log.json",
    "plan_moves.json",
    "plan_keepers.json",
    "atonal_lines.json",
    "reading_book.txt",
    "pattern_book.txt",
];
/// Files that are one piece (never put together): the newer change wins.
const WHOLE: [&str; 3] = ["plan.json", "reading_book.txt", "pattern_book.txt"];
/// library.json's parts each computer keeps to itself: the song files on its own disk (its folders, the ones taken
/// out), its takes, and what it played.
const LOCAL: [&str; 6] = ["files", "folders", "removed_files", "takes", "last", "recent"];
/// Library parts are items named "library/<key>".
const LIB: &str = "library/";
/// How often the two check in with nothing changed (a safety net: a change is sent at once, or asked for by the
/// other's announcements), and how long a step over the network may take.
const HEARTBEAT: Duration = Duration::from_secs(60);
const TIMEOUT: Duration = Duration::from_secs(10);
/// How long a typed code waits for the other computer.
const LINK_WAIT: Duration = Duration::from_secs(60);
/// The most one message may carry.
const MAX: usize = 64 << 20;
/// Kept in sync_backups (the newest).
const BACKUPS: usize = 60;

/// sync.json: on or off, this computer's id and code, and the computer it's linked with.
#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(default)]
struct Saved {
    on: bool,
    id: u64,
    /// the code to type on the other computer (a new one once it's used)
    code: String,
    peer: Option<Peer>,
}

/// The linked computer.
#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(default)]
struct Peer {
    id: u64,
    name: String,
    key: [u64; 2],
    /// its code was typed here (it had the data): the first time, its things come first
    host: bool,
    /// where it syncs ("ip:port"), as last heard
    addr: String,
    /// each item's hash when the two last agreed, as this side has it (its own form of it)
    base: BTreeMap<String, u64>,
    /// when each item last changed here since then (ms since 1970)
    changed: BTreeMap<String, u64>,
    /// when the two last synced (seconds since 1970)
    last: u64,
}

/// What the app is handed: files written (read them again), or library parts to put in (then `library_saved`).
pub(crate) enum Incoming {
    Files(Vec<String>),
    Library(Vec<(String, Value)>),
}

pub(crate) type Sink = Box<dyn Fn(Incoming) + Send + Sync>;

/// What PREFERENCES shows.
#[derive(Clone, Default)]
pub(crate) struct View {
    pub on: bool,
    /// this computer's code (shown while it isn't linked)
    pub code: String,
    /// the linked computer
    pub peer: Option<String>,
    /// when the two last synced (seconds since 1970; 0: not yet)
    pub last: u64,
    /// a code typed here, waiting for the other computer
    pub joining: bool,
    /// what's going on, or what went wrong
    pub note: String,
}

impl View {
    /// The code as it's read out: "482 773".
    pub(crate) fn code_shown(&self) -> String {
        match self.code.len() {
            6 => format!("{} {}", &self.code[..3], &self.code[3..]),
            _ => self.code.clone(),
        }
    }

    /// "Linked with LAPTOP. Synced 2 minutes ago."
    pub(crate) fn linked_line(&self) -> String {
        let name = self.peer.clone().unwrap_or_default();
        if self.last == 0 {
            return format!("Linked with {}. Not synced yet.", name);
        }
        let ago = now_s().saturating_sub(self.last);
        let (n, unit) = match ago {
            0..=59 => return format!("Linked with {}. Synced just now.", name),
            60..=3599 => (ago / 60, "minute"),
            3600..=86399 => (ago / 3600, "hour"),
            _ => (ago / 86400, "day"),
        };
        format!("Linked with {}. Synced {} {}{} ago.", name, n, unit, if n == 1 { "" } else { "s" })
    }
}

static ENGINE: Mutex<Option<Arc<Engine>>> = Mutex::new(None);
static LINKED: AtomicBool = AtomicBool::new(false);

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn engine() -> Option<Arc<Engine>> {
    lock(&ENGINE).clone()
}

/// Syncing with a linked computer (a song that came from it isn't kept here on its own).
pub(crate) fn linked() -> bool {
    LINKED.load(Ordering::Relaxed)
}

/// At start-up, with the app's hand for what comes in: syncing starts if it's on.
pub(crate) fn start(sink: Sink) {
    let dir = crate::api::config_dir();
    let saved: Saved =
        std::fs::read_to_string(dir.join("sync.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    let name = std::env::var("BACKLINE_SYNC_NAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "Backline".to_string());
    let on = saved.on;
    let e = Arc::new(Engine::new(dir, name, saved, sink));
    *lock(&ENGINE) = Some(e.clone());
    if on {
        e.turn_on();
        // (a test copy: a code typed at start)
        if let Ok(code) = std::env::var("BACKLINE_SYNC_LINK") {
            link(&code);
        }
    }
}

/// A test copy's sync stays on this computer (BACKLINE_SYNC_LOCAL): nothing on the network, so no firewall question.
fn local_only() -> bool {
    std::env::var_os("BACKLINE_SYNC_LOCAL").is_some()
}

fn any_addr() -> Ipv4Addr {
    if local_only() {
        Ipv4Addr::LOCALHOST
    } else {
        Ipv4Addr::UNSPECIFIED
    }
}

/// Syncing on or off (PREFERENCES).
pub(crate) fn set_on(on: bool) {
    let Some(e) = engine() else { return };
    // (the threads of the last start leave)
    e.gen.fetch_add(1, Ordering::SeqCst);
    if on {
        e.turn_on();
    } else {
        let mut s = lock(&e.saved);
        s.on = false;
        e.write(&s);
        LINKED.store(false, Ordering::Relaxed);
    }
}

/// Link with the computer showing this code: it's called, or it calls here (for a minute).
pub(crate) fn link(code: &str) {
    if let Some(e) = engine() {
        *lock(&e.joining) = Some((digits(code), Instant::now() + LINK_WAIT, random()));
        e.set_note("Looking for the other computer...");
        e.ask(|w| {
            w.say = true;
            w.poke = true;
        });
    }
}

/// Stop syncing with the linked computer (link again with a code).
pub(crate) fn unlink() {
    let Some(e) = engine() else { return };
    let mut s = lock(&e.saved);
    s.peer = None;
    s.code = new_code();
    e.write(&s);
    LINKED.store(false, Ordering::Relaxed);
    drop(s);
    e.set_note("");
}

/// Sync now, without waiting for the next check-in.
pub(crate) fn sync_now() {
    if let Some(e) = engine() {
        e.ask(|w| w.poke = true);
    }
}

/// The app has put in the library parts it was handed and saved them (`ok`), or couldn't: then the library isn't
/// synced till Backline starts again, so what's here never goes over the other computer's.
pub(crate) fn library_saved(ok: bool) {
    if let Some(e) = engine() {
        *lock(&e.lib_wait) = None;
        if !ok {
            e.lib_off.store(true, Ordering::Relaxed);
            e.set_note("The library from your other computer didn't fit this version of Backline: update Backline on both.");
        }
    }
}

pub(crate) fn view() -> View {
    let Some(e) = engine() else { return View::default() };
    let (joining, note) = (e.joining_code().is_some(), lock(&e.note).clone());
    let s = lock(&e.saved);
    let v = View {
        on: s.on,
        code: s.code.clone(),
        peer: s.peer.as_ref().map(|p| p.name.clone()),
        last: s.peer.as_ref().map_or(0, |p| p.last),
        joining,
        note,
    };
    v
}

/// What's here, as last read: each item's hash and text, items unreadable just now ("library/": all of the library),
/// the files' times (read again when they change), and each item's hash at the last look (a change since then is a
/// change made here).
#[derive(Default)]
struct Snap {
    read: bool,
    stamps: BTreeMap<&'static str, Option<(SystemTime, u64)>>,
    items: BTreeMap<String, (u64, String)>,
    skip: BTreeSet<String>,
    seen: BTreeMap<String, u64>,
}

/// Another Backline, as it announces itself.
#[derive(Clone)]
struct Near {
    /// where it syncs
    addr: SocketAddr,
    linked: bool,
    /// a code typed there, and which typing
    joining: Option<u64>,
}

#[derive(Default)]
struct Wake {
    poke: bool,
    say: bool,
}

/// Why a round didn't go through: the other side was in one of its own, couldn't be reached, or said why not.
enum Fail {
    Busy,
    Net(String),
    Said(String),
}

impl From<String> for Fail {
    fn from(s: String) -> Fail {
        Fail::Net(s)
    }
}

struct Engine {
    dir: PathBuf,
    name: String,
    saved: Mutex<Saved>,
    sink: Sink,
    /// threads of an older start leave (turned off, or on again)
    gen: AtomicU64,
    port: AtomicU16,
    /// wrong codes typed against this computer's code (5: a new code)
    tries: Mutex<u32>,
    /// a code typed here, till when it waits, and which typing it is
    joining: Mutex<Option<(String, Instant, u64)>>,
    /// Backlines heard lately, and the typings this one has called about
    near: Mutex<HashMap<u64, Near>>,
    called: Mutex<BTreeSet<(u64, u64)>>,
    /// one link at a time, one round at a time (calling or called)
    linking: Mutex<()>,
    round: Mutex<()>,
    snap: Mutex<Snap>,
    /// something here the other computer doesn't have yet
    dirty: AtomicBool,
    /// the linked computer heard since start
    met: AtomicBool,
    /// library parts handed to the app and not saved by it yet; the app couldn't take them (no library sync); the
    /// parts taken, to read again as the app saved them
    lib_wait: Mutex<Option<Instant>>,
    lib_off: AtomicBool,
    took_lib: Mutex<BTreeSet<String>>,
    /// items the other side wants the text of next time
    want: Mutex<BTreeSet<String>>,
    /// the last round that went through, either way
    ok_at: Mutex<Option<Instant>>,
    note: Mutex<String>,
    wake: (Mutex<Wake>, Condvar),
}

impl Engine {
    fn new(dir: PathBuf, name: String, saved: Saved, sink: Sink) -> Engine {
        Engine {
            dir,
            name,
            saved: Mutex::new(saved),
            sink,
            gen: AtomicU64::new(0),
            port: AtomicU16::new(0),
            tries: Mutex::new(0),
            joining: Mutex::new(None),
            near: Mutex::new(HashMap::new()),
            called: Mutex::new(BTreeSet::new()),
            linking: Mutex::new(()),
            round: Mutex::new(()),
            snap: Mutex::new(Snap::default()),
            dirty: AtomicBool::new(false),
            met: AtomicBool::new(false),
            lib_wait: Mutex::new(None),
            lib_off: AtomicBool::new(false),
            took_lib: Mutex::new(BTreeSet::new()),
            want: Mutex::new(BTreeSet::new()),
            ok_at: Mutex::new(None),
            note: Mutex::new(String::new()),
            wake: (Mutex::new(Wake::default()), Condvar::new()),
        }
    }

    fn id(&self) -> u64 {
        lock(&self.saved).id
    }

    fn peer(&self) -> Option<Peer> {
        lock(&self.saved).peer.clone()
    }

    fn write(&self, s: &Saved) {
        if let Ok(t) = serde_json::to_string_pretty(s) {
            let _ = write_whole(&self.dir.join("sync.json"), &t);
        }
    }

    fn live(&self, g: u64) -> bool {
        self.gen.load(Ordering::SeqCst) == g
    }

    fn ask(&self, f: impl FnOnce(&mut Wake)) {
        f(&mut lock(&self.wake.0));
        self.wake.1.notify_all();
    }

    fn set_note(&self, s: &str) {
        *lock(&self.note) = s.to_string();
    }

    /// The code typed here, while it still waits.
    fn joining_code(&self) -> Option<String> {
        lock(&self.joining).as_ref().filter(|j| Instant::now() < j.1).map(|j| j.0.clone())
    }

    fn turn_on(self: &Arc<Self>) {
        {
            let mut s = lock(&self.saved);
            s.on = true;
            if s.id == 0 {
                s.id = random().max(1);
            }
            if s.code.len() != 6 {
                s.code = new_code();
            }
            self.write(&s);
            LINKED.store(s.peer.is_some(), Ordering::Relaxed);
        }
        let g = self.gen.load(Ordering::SeqCst);
        let jobs: [fn(Arc<Engine>, u64); 3] = [Engine::listen, Engine::find, Engine::work];
        for job in jobs {
            let e = self.clone();
            let _ = std::thread::Builder::new().name("sync".to_string()).spawn(move || job(e, g));
        }
    }

    /// Takes the other computer's calls: a round, or linking.
    fn listen(self: Arc<Self>, g: u64) {
        let l = TcpListener::bind((any_addr(), SYNC_PORT)).or_else(|_| TcpListener::bind((any_addr(), 0)));
        let l = match l {
            Ok(l) => l,
            Err(err) => {
                self.set_note(&format!("Can't use the network: {}", err));
                return;
            }
        };
        let _ = l.set_nonblocking(true);
        self.port.store(l.local_addr().map_or(0, |a| a.port()), Ordering::Relaxed);
        if std::env::var_os("BACKLINE_SYNC_NO_CALLS").is_some() {
            return;
        }
        while self.live(g) {
            match l.accept() {
                Ok((s, _)) => {
                    let _ = s.set_nonblocking(false);
                    let e = self.clone();
                    std::thread::spawn(move || {
                        if let Err(why) = e.serve(s) {
                            log(&why);
                        }
                    });
                }
                Err(err) if err.kind() == ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(150)),
                Err(_) => std::thread::sleep(Duration::from_millis(500)),
            }
        }
    }

    /// Says where this computer syncs, every few seconds, to the whole network (and to the linked computer where it
    /// was last), and hears the others. An announcement heard is answered straight back, so a computer that can't
    /// hear broadcasts (its firewall) still hears of this one.
    fn find(self: Arc<Self>, g: u64) {
        let sock = UdpSocket::bind((any_addr(), FIND_PORT)).or_else(|_| UdpSocket::bind((any_addr(), 0)));
        let Ok(sock) = sock else { return };
        let _ = sock.set_broadcast(true);
        let _ = sock.set_read_timeout(Some(Duration::from_millis(300)));
        let mut said: Option<Instant> = None;
        let mut answered: HashMap<u64, Instant> = HashMap::new();
        let mut buf = [0u8; 600];
        while self.live(g) {
            let say_now = std::mem::take(&mut lock(&self.wake.0).say);
            if self.port.load(Ordering::Relaxed) != 0 && (say_now || said.map_or(true, |t| t.elapsed() > Duration::from_secs(4))) {
                said = Some(Instant::now());
                let msg = self.hello(false);
                let peer_ip = self.peer().and_then(|p| p.addr.parse::<SocketAddr>().ok()).map(|a| a.ip());
                for to in targets(peer_ip) {
                    let _ = sock.send_to(msg.as_bytes(), to);
                }
            }
            let Ok((n, from)) = sock.recv_from(&mut buf) else { continue };
            let Some(h) = parse_hello(&buf[..n]) else { continue };
            if h.id == self.id() || h.port == 0 {
                continue;
            }
            if !h.reply && answered.get(&h.id).map_or(true, |t| t.elapsed() > Duration::from_secs(2)) {
                answered.insert(h.id, Instant::now());
                let _ = sock.send_to(self.hello(true).as_bytes(), from);
            }
            self.heard(h, from.ip());
        }
    }

    /// This computer's announcement: "BLSYNC1|id|port|flags|joining|name" (flags: L linked, D something new to sync,
    /// R an answer).
    fn hello(&self, reply: bool) -> String {
        let s = lock(&self.saved);
        let mut f = String::new();
        if s.peer.is_some() {
            f.push('L');
        }
        if self.dirty.load(Ordering::Relaxed) {
            f.push('D');
        }
        if reply {
            f.push('R');
        }
        let joining = lock(&self.joining).as_ref().filter(|j| Instant::now() < j.1).map_or(0, |j| j.2);
        format!("BLSYNC1|{}|{}|{}|{}|{}", s.id, self.port.load(Ordering::Relaxed), f, joining, self.name)
    }

    /// Another Backline heard: the linked one's address is kept (and it's synced with when it has something new, or
    /// first heard); one where a code was just typed is called, in case the code is this computer's.
    fn heard(self: &Arc<Self>, h: Hello, ip: IpAddr) {
        let addr = SocketAddr::new(ip, h.port);
        lock(&self.near).insert(h.id, Near { addr, linked: h.linked, joining: h.joining });
        let (mine, unlinked) = {
            let s = lock(&self.saved);
            (s.peer.as_ref().is_some_and(|p| p.id == h.id), s.on && s.peer.is_none())
        };
        if mine {
            // (kept with the next round: a computer heard at two addresses doesn't rewrite sync.json each time)
            if let Some(p) = lock(&self.saved).peer.as_mut() {
                p.addr = addr.to_string();
                p.name = h.name.clone();
            }
            if h.dirty || !self.met.swap(true, Ordering::Relaxed) {
                self.ask(|w| w.poke = true);
            }
        } else if let Some(j) = h.joining.filter(|_| unlinked && !h.linked && self.joining_code().is_none()) {
            // (each typing called about until it answers: its firewall may not have been allowed yet)
            if lock(&self.called).insert((h.id, j)) {
                let e = self.clone();
                std::thread::spawn(move || {
                    if let Err((answered, why)) = e.call_join(addr, true) {
                        log(&why);
                        if !answered {
                            lock(&e.called).remove(&(h.id, j));
                        }
                    }
                });
            }
        }
    }

    /// Links when a code was typed here, looks for changes here once a second, and syncs: at once when something
    /// changed (or the linked computer has something new, or was just heard), and every HEARTBEAT anyway.
    fn work(self: Arc<Self>, g: u64) {
        let mut last: Option<Instant> = None;
        let mut fails = 0u32;
        let mut tried: BTreeSet<SocketAddr> = BTreeSet::new();
        let mut typing = 0u64;
        while self.live(g) {
            let poke = std::mem::take(&mut lock(&self.wake.0).poke);
            let j = lock(&self.joining).clone();
            match j {
                Some((_, until, t)) if Instant::now() < until => {
                    if t != typing {
                        typing = t;
                        tried.clear();
                    }
                    self.try_join(&mut tried);
                }
                Some(_) => {
                    *lock(&self.joining) = None;
                    if self.peer().is_none() {
                        self.set_note("No other computer answered. Turn SYNC on there, on the same network, and type its code again.");
                    }
                }
                None => {}
            }
            if let Some(peer) = self.peer() {
                let changed = self.poll();
                let wait = if fails > 0 {
                    Duration::from_secs(15)
                } else if changed {
                    Duration::ZERO
                } else {
                    HEARTBEAT
                };
                if poke || last.map_or(true, |t| t.elapsed() >= wait) {
                    last = Some(Instant::now());
                    let addr = lock(&self.near).get(&peer.id).map(|n| n.addr).or_else(|| peer.addr.parse().ok());
                    match addr.map_or(Err(Fail::Net(String::new())), |a| self.round(a)) {
                        Ok(()) => fails = 0,
                        Err(Fail::Busy) => {
                            // (the other side is in a round of its own: again in a moment)
                            std::thread::sleep(Duration::from_millis(200 + random() % 600));
                            last = None;
                        }
                        Err(Fail::Said(why)) => {
                            fails += 1;
                            self.set_note(&why);
                        }
                        Err(Fail::Net(why)) => {
                            fails += 1;
                            if !why.is_empty() && fails == 1 {
                                log(&format!("{} can't be reached: {}", peer.name, why));
                            }
                        }
                    }
                }
                // (it may still sync by calling here)
                if fails > 1 && lock(&self.ok_at).map_or(true, |t| t.elapsed() > Duration::from_secs(60)) {
                    self.set_note(&format!(
                        "Can't reach {} right now. SYNC needs to be on there too, on the same network.",
                        peer.name
                    ));
                }
            }
            let w = lock(&self.wake.0);
            let _ = self.wake.1.wait_timeout_while(w, Duration::from_secs(1), |w| !w.poke && self.live(g));
        }
    }

    /// While a code typed here waits: calls each Backline heard that isn't linked or linking (each once; the other
    /// computer may call here instead).
    fn try_join(&self, tried: &mut BTreeSet<SocketAddr>) {
        let hosts: Vec<SocketAddr> =
            lock(&self.near).values().filter(|n| !n.linked && n.joining.is_none()).map(|n| n.addr).collect();
        for a in hosts {
            if self.peer().is_some() || self.joining_code().is_none() {
                return;
            }
            if tried.insert(a) {
                if let Err((_, why)) = self.call_join(a, false) {
                    log(&why);
                }
            }
        }
    }

    /// Calls a computer to link: this one the host (a code was typed there) or the one linking (a code typed here).
    /// An error says whether the other computer answered at all.
    fn call_join(&self, addr: SocketAddr, me_host: bool) -> Result<String, (bool, String)> {
        let unanswered = |e: String| (false, e);
        let s = TcpStream::connect_timeout(&addr, Duration::from_secs(3)).map_err(|e| unanswered(e.to_string()))?;
        let mut c = Conn::new(s, 0);
        let mine = random();
        c.send(&Wire::Join { id: self.id(), name: self.name.clone(), nonce: mine, host: me_host }).map_err(unanswered)?;
        match c.recv().map_err(unanswered)? {
            Wire::Join { id, name, nonce, host } if host != me_host => {
                self.link_talk(&mut c, me_host, mine, id, name, nonce).map_err(|e| (true, e))
            }
            Wire::No { why } => Err((true, why)),
            _ => Err((true, "an unexpected answer".to_string())),
        }
    }

    /// A call: a round, or linking.
    fn serve(&self, s: TcpStream) -> Result<(), String> {
        let mut c = Conn::new(s, 1);
        match c.recv()? {
            Wire::Hello { id, nonce } => self.serve_round(c, id, nonce),
            Wire::Join { id, name, nonce, host } => {
                let me_host = !host;
                let ready = if me_host { lock(&self.saved).peer.is_none() } else { self.joining_code().is_some() };
                if !ready {
                    let why = if me_host { format!("{} is already linked", self.name) } else { "Type the code again".to_string() };
                    return c.send(&Wire::No { why });
                }
                let mine = random();
                c.send(&Wire::Join { id: self.id(), name: self.name.clone(), nonce: mine, host: me_host })?;
                self.link_talk(&mut c, me_host, mine, id, name, nonce).map(|_| ())
            }
            _ => Err("an unexpected call".to_string()),
        }
    }

    /// The rest of linking, whichever side called: the one linking proves it has the code, the host proves it too
    /// and hands over a new key (hidden with the code). Both keep the other as linked.
    fn link_talk(&self, c: &mut Conn, me_host: bool, mine: u64, id: u64, name: String, theirs: u64) -> Result<String, String> {
        let _one = lock(&self.linking);
        let (nj, nh) = if me_host { (theirs, mine) } else { (mine, theirs) };
        if me_host {
            let m = match c.recv()? {
                Wire::Answer { mac: m } => m,
                Wire::No { why } => return Err(why),
                _ => return Err("an unexpected answer".to_string()),
            };
            let code = lock(&self.saved).code.clone();
            let ck = code_key(&code);
            if lock(&self.saved).peer.is_some() {
                let _ = c.send(&Wire::No { why: format!("{} is already linked", self.name) });
                return Err("already linked".to_string());
            }
            if m != mac(ck, &[b"join", &le(nj), &le(nh)]) {
                let mut t = lock(&self.tries);
                *t += 1;
                if *t >= 5 {
                    // (too many tries: a new code)
                    *t = 0;
                    let mut s = lock(&self.saved);
                    s.code = new_code();
                    self.write(&s);
                }
                let _ = c.send(&Wire::No { why: "That code doesn't match".to_string() });
                return Err(format!("{} typed a code that doesn't match", name));
            }
            let key = [random(), random()];
            let pad = pad(ck, nj, nh);
            c.send(&Wire::Welcome { mac: mac(ck, &[b"host", &le(nh), &le(nj)]), key: [key[0] ^ pad[0], key[1] ^ pad[1]] })?;
            self.linked_with(Peer { id, name: name.clone(), key, host: false, ..Default::default() });
        } else {
            let Some(code) = self.joining_code().filter(|_| self.peer().is_none()) else {
                let _ = c.send(&Wire::No { why: "Type the code again".to_string() });
                return Err("no code typed".to_string());
            };
            let ck = code_key(&code);
            c.send(&Wire::Answer { mac: mac(ck, &[b"join", &le(nj), &le(nh)]) })?;
            match c.recv()? {
                Wire::Welcome { mac: m, key } if m == mac(ck, &[b"host", &le(nh), &le(nj)]) => {
                    let pad = pad(ck, nj, nh);
                    self.linked_with(Peer { id, name: name.clone(), key: [key[0] ^ pad[0], key[1] ^ pad[1]], host: true, ..Default::default() });
                }
                Wire::No { why } => {
                    if why.contains("doesn't match") {
                        *lock(&self.joining) = None;
                        self.set_note("That code doesn't match: check the code on the other computer.");
                    }
                    return Err(why);
                }
                _ => return Err("an unexpected answer".to_string()),
            }
        }
        Ok(name)
    }

    /// Linked: the other computer kept, a new code for next time, and a sync at once.
    fn linked_with(&self, p: Peer) {
        log(&format!("linked with {}", p.name));
        {
            let mut s = lock(&self.saved);
            s.code = new_code();
            s.peer = Some(p);
            self.write(&s);
        }
        *lock(&self.joining) = None;
        *lock(&self.tries) = 0;
        // (a fresh look, against nothing agreed yet)
        *lock(&self.snap) = Snap::default();
        LINKED.store(true, Ordering::Relaxed);
        self.set_note("");
        self.ask(|w| w.poke = true);
    }

    /// One round, this side calling: what's here goes over (with the text of what changed here), the other side
    /// decides, takes what it takes, and says what to take here.
    fn round(&self, addr: SocketAddr) -> Result<(), Fail> {
        let s = TcpStream::connect_timeout(&addr, Duration::from_secs(3)).map_err(|e| e.to_string())?;
        // (one at a time, once it's through: a call that can't get through doesn't turn the other's calls away)
        let _one = lock(&self.round);
        self.poll();
        let peer = self.peer().ok_or_else(|| Fail::Said("Not linked".to_string()))?;
        let fresh = fresh(&peer);
        let mut c = Conn::new(s, 0);
        let mine = random();
        c.send(&Wire::Hello { id: self.id(), nonce: mine })?;
        let theirs = match c.recv()? {
            Wire::Proof { nonce, mac: m } if m == mac(peer.key, &[b"proof", &le(mine), &le(nonce)]) => nonce,
            Wire::Proof { .. } => return Err(Fail::Said(format!("{} isn't linked with this computer any more: link them again.", peer.name))),
            Wire::Busy => return Err(Fail::Busy),
            Wire::No { why } => return Err(Fail::Said(why)),
            _ => return Err(Fail::Net("an unexpected answer".to_string())),
        };
        c.key = Some(session(peer.key, mine, theirs));
        let want = std::mem::take(&mut *lock(&self.want));
        let lib_off = self.lib_off.load(Ordering::Relaxed);
        let (items, mut skip) = {
            let sn = lock(&self.snap);
            let names: BTreeSet<&String> = sn.items.keys().chain(peer.base.keys()).collect();
            let mut items = Vec::new();
            for n in names {
                if skipped(&sn.skip, n) {
                    continue;
                }
                let (h, text) = match sn.items.get(n) {
                    Some((h, t)) if !fresh => (*h, Some(t)),
                    _ => (0, None),
                };
                let b = peer.base.get(n).copied().unwrap_or(0);
                let send = h != 0 && (h != b || b == 0 || want.contains(n));
                let at = peer.changed.get(n).copied().unwrap_or(0);
                items.push(Have { name: n.clone(), hash: h, base: b, at, text: if send { text.cloned() } else { None } });
            }
            (items, sn.skip.iter().cloned().collect::<Vec<_>>())
        };
        if lib_off {
            skip.push(LIB.to_string());
        }
        c.send(&Wire::State { items, skip })?;
        let (put, agreed, want) = match c.recv()? {
            Wire::Plan { put, agreed, want } => (put, agreed, want),
            Wire::No { why } => return Err(Fail::Said(why)),
            _ => return Err(Fail::Net("an unexpected answer".to_string())),
        };
        let took: Vec<(String, String, bool)> = put.into_iter().map(|p| (p.name, p.text, p.keep || fresh)).collect();
        self.take(&took);
        c.send(&Wire::Ack)?;
        self.agree(&agreed);
        if !took.is_empty() {
            log(&format!("took {} from {}: {}", took.len(), peer.name, names_of(&took)));
        }
        if !want.is_empty() {
            // (it needs the text of these: at once)
            *lock(&self.want) = want.into_iter().collect();
            self.ask(|w| w.poke = true);
        }
        Ok(())
    }

    /// A round, the other side calling: what's there is weighed against what's here, item by item; what this side
    /// takes is put in place, then the other is told what to take.
    fn serve_round(&self, mut c: Conn, id: u64, theirs: u64) -> Result<(), String> {
        let Some(peer) = self.peer().filter(|p| p.id == id) else {
            let why = format!("{} isn't linked with this computer any more: link them again.", self.name);
            return c.send(&Wire::No { why });
        };
        let Ok(_one) = self.round.try_lock() else { return c.send(&Wire::Busy) };
        let mine = random();
        c.send(&Wire::Proof { nonce: mine, mac: mac(peer.key, &[b"proof", &le(theirs), &le(mine)]) })?;
        c.key = Some(session(peer.key, theirs, mine));
        let (have, their_skip) = match c.recv()? {
            Wire::State { items, skip } => (items, skip.into_iter().collect::<BTreeSet<String>>()),
            _ => return Err("an unexpected message".to_string()),
        };
        self.poll();
        let peer = self.peer().unwrap_or(peer);
        let fresh = fresh(&peer);
        let host = !peer.host;
        let lower = self.id() < id;
        let lib_off = self.lib_off.load(Ordering::Relaxed);
        let their: HashMap<&str, &Have> = have.iter().map(|h| (h.name.as_str(), h)).collect();
        let (mut put, mut agreed, mut want, mut took) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        {
            let sn = lock(&self.snap);
            let names: BTreeSet<&str> =
                sn.items.keys().chain(peer.base.keys()).map(|s| s.as_str()).chain(their.keys().copied()).collect();
            for n in names {
                if skipped(&sn.skip, n) || skipped(&their_skip, n) || (lib_off && n.starts_with(LIB)) || !item_ok(n) {
                    continue;
                }
                let mine_x = sn.items.get(n).filter(|_| !fresh);
                let m = mine_x.map_or(0, |x| x.0);
                let th = their.get(n);
                let t = th.map_or(0, |h| h.hash);
                let mb = peer.base.get(n).copied().unwrap_or(0);
                let tb = th.map_or(0, |h| h.base);
                let my_at = peer.changed.get(n).copied().unwrap_or(0);
                let their_text = th.and_then(|h| h.text.clone()).filter(|x| hash(x) == t);
                match decide(n, m, t, mb, tb, my_at, th.map_or(0, |h| h.at), host, lower) {
                    Pick::Same => {
                        // (the calling side's own form: a Backline from before own forms takes this as its base)
                        if m != 0 {
                            agreed.push((n.to_string(), if t != 0 { t } else { m }));
                        }
                    }
                    Pick::Mine { keep } => {
                        if let Some((_, text)) = mine_x {
                            put.push(Put { name: n.to_string(), text: text.clone(), keep });
                            agreed.push((n.to_string(), m));
                        }
                    }
                    Pick::Theirs { keep } => match their_text {
                        Some(text) => {
                            took.push((n.to_string(), text, keep || fresh));
                            agreed.push((n.to_string(), t));
                        }
                        None => want.push(n.to_string()),
                    },
                    Pick::Merge { mine_first } => {
                        let parsed = (
                            mine_x.and_then(|x| serde_json::from_str::<Value>(&x.1).ok()),
                            their_text.and_then(|x| serde_json::from_str::<Value>(&x).ok()),
                        );
                        let (Some(a), Some(b)) = parsed else {
                            want.push(n.to_string());
                            continue;
                        };
                        let v = if mine_first { merge(&a, &b, 0) } else { merge(&b, &a, 0) };
                        let text = if n.starts_with(LIB) { v.to_string() } else { serde_json::to_string_pretty(&v).unwrap_or_default() };
                        let h = hash(&text);
                        if h != m {
                            took.push((n.to_string(), text.clone(), true));
                        }
                        if h != t {
                            put.push(Put { name: n.to_string(), text, keep: true });
                        }
                        agreed.push((n.to_string(), h));
                    }
                }
            }
        }
        self.take(&took);
        let sent = put.len();
        c.send(&Wire::Plan { put, agreed: agreed.clone(), want })?;
        if let Ok(Wire::Ack) = c.recv() {
            self.agree(&agreed);
        }
        if !took.is_empty() || sent > 0 {
            log(&format!("{} called: took {} ({}), sent {}", peer.name, took.len(), names_of(&took), sent));
        }
        Ok(())
    }

    /// Reads what's here again when one of its files changed since the last look, and stamps when each item changed
    /// here (an item that differs from what the two last agreed). True when something changed here since the last
    /// look.
    fn poll(&self) -> bool {
        let hold = lock(&self.lib_wait).is_some_and(|t| t.elapsed() < Duration::from_secs(30));
        let mut st = stamps(&self.dir);
        if hold {
            st.insert("library.json", None);
        }
        let mut sn = lock(&self.snap);
        if sn.read && sn.stamps == st {
            return false;
        }
        let first = !sn.read;
        let (mut items, mut skip) = read_here(&self.dir, hold);
        if hold {
            // (the parts that came in, till the app has saved them)
            for (n, x) in sn.items.iter().filter(|(n, _)| n.starts_with(LIB)) {
                items.insert(n.clone(), x.clone());
            }
            if sn.skip.contains(LIB) {
                skip.insert(LIB.to_string());
            }
        }
        sn.items = items;
        sn.skip = skip;
        sn.stamps = st;
        sn.read = true;
        let mut s = lock(&self.saved);
        let Some(p) = s.peer.as_mut() else { return false };
        // (library parts that came in, as the app saved them here: what the two agreed on, in this side's form)
        if !hold {
            for n in std::mem::take(&mut *lock(&self.took_lib)) {
                if let Some(h) = sn.items.get(&n).map(|x| x.0) {
                    p.base.insert(n.clone(), h);
                    sn.seen.insert(n, h);
                }
            }
        }
        let names: BTreeSet<String> = sn.items.keys().chain(sn.seen.keys()).chain(p.base.keys()).cloned().collect();
        let now = now_ms();
        let mut changed = false;
        let mut dirty = false;
        for n in names {
            if skipped(&sn.skip, &n) {
                continue;
            }
            let h = sn.items.get(&n).map_or(0, |x| x.0);
            let differs = p.base.get(&n).copied().unwrap_or(0) != h;
            dirty |= differs;
            if !first && sn.seen.get(&n).copied().unwrap_or(0) == h {
                continue;
            }
            sn.seen.insert(n.clone(), h);
            if differs {
                // (on the first look, a change from before: when its file was last written)
                if first {
                    let at = modified_ms(&self.dir, &n).unwrap_or(now);
                    p.changed.entry(n).or_insert(at);
                } else {
                    p.changed.insert(n, now);
                }
                changed = true;
            }
        }
        self.dirty.store(dirty, Ordering::Relaxed);
        if changed {
            self.write(&s);
        }
        changed
    }

    /// Puts texts that came in in place: a file written whole, library parts handed to the app (library.json isn't
    /// read again till it has saved them). The old one goes to sync_backups first when asked. Names that aren't items
    /// are ignored.
    fn take(&self, items: &[(String, String, bool)]) {
        let mut files = Vec::new();
        let mut lib = Vec::new();
        let mut sn = lock(&self.snap);
        for (name, text, keep) in items {
            if !item_ok(name) {
                continue;
            }
            // (a JSON item that doesn't read is left alone; a book is plain text)
            let v = serde_json::from_str::<Value>(text).ok();
            if v.is_none() && !name.ends_with(".txt") {
                continue;
            }
            if *keep {
                if let Some((_, old)) = sn.items.get(name) {
                    self.backup(name, old);
                }
            }
            if let Some(key) = name.strip_prefix(LIB) {
                let mut v = v.unwrap_or(Value::Null);
                // (fields only this side knows, kept)
                if let Some(ours) = sn.items.get(name).and_then(|(_, t)| serde_json::from_str::<Value>(t).ok()) {
                    keep_ours(&mut v, &ours);
                }
                lock(&self.took_lib).insert(name.clone());
                lib.push((key.to_string(), v));
            } else if let Some(f) = FILES.iter().copied().find(|f| *f == name.as_str()) {
                let path = self.dir.join(f);
                if let Err(e) = write_whole(&path, text) {
                    log(&format!("couldn't write {}: {}", f, e));
                    continue;
                }
                sn.stamps.insert(f, stamp(&path));
                files.push(name.clone());
            }
            let h = hash(text);
            sn.items.insert(name.clone(), (h, text.clone()));
            sn.seen.insert(name.clone(), h);
        }
        drop(sn);
        if !lib.is_empty() {
            *lock(&self.lib_wait) = Some(Instant::now());
            (self.sink)(Incoming::Library(lib));
        }
        if !files.is_empty() {
            (self.sink)(Incoming::Files(files));
        }
    }

    /// The round went through: what both agree on is the new base, and nothing is waiting to go across from here.
    fn agree(&self, agreed: &[(String, u64)]) {
        {
            let sn = lock(&self.snap);
            let mut s = lock(&self.saved);
            if let Some(p) = s.peer.as_mut() {
                for (n, _) in agreed {
                    // (what's here now, in this side's own form)
                    p.base.insert(n.clone(), sn.items.get(n).map_or(0, |x| x.0));
                    p.changed.remove(n);
                }
                p.last = now_s();
                let dirty = sn.items.iter().any(|(n, x)| !skipped(&sn.skip, n) && p.base.get(n).copied().unwrap_or(0) != x.0);
                self.dirty.store(dirty, Ordering::Relaxed);
            }
            self.write(&s);
        }
        *lock(&self.ok_at) = Some(Instant::now());
        let mut note = lock(&self.note);
        if !note.starts_with("The library") {
            note.clear();
        }
    }

    /// An item about to be replaced, kept in sync_backups (the newest BACKUPS).
    fn backup(&self, name: &str, text: &str) {
        let d = self.dir.join("sync_backups");
        if std::fs::create_dir_all(&d).is_err() {
            return;
        }
        let _ = std::fs::write(d.join(format!("{}.{}.txt", name.replace('/', "-"), now_ms())), text);
        if let Ok(rd) = std::fs::read_dir(&d) {
            let mut v: Vec<(SystemTime, PathBuf)> =
                rd.flatten().filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path()))).collect();
            if v.len() > BACKUPS {
                v.sort();
                for (_, p) in &v[..v.len() - BACKUPS] {
                    let _ = std::fs::remove_file(p);
                }
            }
        }
    }
}

/// The first round on the computer where the code was typed: what's here doesn't count, the other computer's things
/// come over as they are (what was here is kept in sync_backups first).
fn fresh(p: &Peer) -> bool {
    p.host && p.base.is_empty()
}

/// What to do with one item, from the side that decides: Same, this side's goes across (`keep`: the other side keeps
/// its old one first), the other side's comes here, or both put together (the first one's way where they differ).
#[derive(Debug, PartialEq, Clone, Copy)]
enum Pick {
    Same,
    Mine { keep: bool },
    Theirs { keep: bool },
    Merge { mine_first: bool },
}

/// m and t: the item's hash on each side (0: not there); mb and tb: what each side last agreed on (0: never); when
/// each changed it; whether this side is the host (its code was typed on the other); whether its id is the lower.
#[allow(clippy::too_many_arguments)]
fn decide(name: &str, m: u64, t: u64, mb: u64, tb: u64, my_at: u64, their_at: u64, host: bool, lower: bool) -> Pick {
    if m == t {
        return Pick::Same;
    }
    // (never taken away: the side without it gets it)
    if t == 0 {
        return Pick::Mine { keep: false };
    }
    if m == 0 {
        return Pick::Theirs { keep: false };
    }
    let first = mb == 0 || tb == 0;
    // (neither changed since the two agreed, each in its own form)
    if !first && m == mb && t == tb {
        return Pick::Same;
    }
    if !first && m == mb {
        return Pick::Theirs { keep: false };
    }
    if !first && t == tb {
        return Pick::Mine { keep: false };
    }
    // changed on both, or never agreed: the newer change first (the first time, the host's)
    let mine_first = if first { host } else { my_at > their_at || (my_at == their_at && lower) };
    if WHOLE.contains(&name) {
        if mine_first {
            Pick::Mine { keep: true }
        } else {
            Pick::Theirs { keep: true }
        }
    } else {
        Pick::Merge { mine_first }
    }
}

/// Two versions put together, `a` first: maps get both sides' keys, lists both sides' entries (an entry once: the
/// same id, name, date, link or text, or the same whole), and where both have something else, a's. Two levels down
/// at most (below that, a's whole).
fn merge(a: &Value, b: &Value, depth: u32) -> Value {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) if depth <= 2 => {
            let mut out = x.clone();
            for (k, v) in y {
                let m = match out.get(k) {
                    Some(o) => merge(o, v, depth + 1),
                    None => v.clone(),
                };
                out.insert(k.clone(), m);
            }
            Value::Object(out)
        }
        (Value::Array(x), Value::Array(y)) if depth <= 2 => {
            let mut out = x.clone();
            let mut ids: Vec<String> = out.iter().map(ident).collect();
            for v in y {
                let id = ident(v);
                match ids.iter().position(|i| *i == id) {
                    Some(i) => out[i] = merge(&out[i], v, depth + 1),
                    None => {
                        out.push(v.clone());
                        ids.push(id);
                    }
                }
            }
            Value::Array(out)
        }
        _ => a.clone(),
    }
}

/// Fields of ours the other side's version doesn't have, put into what came from it (a newer Backline's: a diary
/// day's notes), so an older Backline never erases them. Only a record's fields (its keys plain words: `note`,
/// `loop_time`), in the part itself or in the same entry of a list of records (the same id, name, date...), never a
/// map's own keys (a song's bookmarks taken out stay out).
fn keep_ours(theirs: &mut Value, ours: &Value) {
    fn field(k: &str) -> bool {
        k.starts_with(|c: char| c.is_ascii_lowercase()) && k.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    }
    fn record(t: &mut serde_json::Map<String, Value>, o: &serde_json::Map<String, Value>) {
        if !t.keys().chain(o.keys()).all(|k| field(k)) {
            return;
        }
        for (k, v) in o {
            if !t.contains_key(k) {
                t.insert(k.clone(), v.clone());
            }
        }
    }
    match (theirs, ours) {
        (Value::Object(t), Value::Object(o)) => record(t, o),
        (Value::Array(t), Value::Array(o)) => {
            for x in t.iter_mut() {
                let id = ident(x);
                if let (Value::Object(xm), Some(Value::Object(om))) = (x, o.iter().find(|v| ident(v) == id)) {
                    record(xm, om);
                }
            }
        }
        _ => {}
    }
}

/// What makes two list entries the same thing.
fn ident(v: &Value) -> String {
    if let Value::Object(m) = v {
        for k in ["id", "name", "date", "url", "text"] {
            if let Some(x) = m.get(k) {
                return format!("{}={}", k, x);
            }
        }
    }
    v.to_string()
}

/// What's here: each item's hash and text, and items that can't be read just now (a JSON file being written; the
/// library when `hold` keeps the last parts). Never a song: only the files that sync and library.json's own parts.
fn read_here(dir: &Path, hold: bool) -> (BTreeMap<String, (u64, String)>, BTreeSet<String>) {
    let mut items = BTreeMap::new();
    let mut skip = BTreeSet::new();
    for f in FILES {
        match std::fs::read_to_string(dir.join(f)) {
            Ok(text) => {
                if f.ends_with(".json") && serde_json::from_str::<Value>(&text).is_err() {
                    skip.insert(f.to_string());
                } else {
                    items.insert(f.to_string(), (hash(&text), text));
                }
            }
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Err(_) => {
                skip.insert(f.to_string());
            }
        }
    }
    if !hold {
        match std::fs::read_to_string(dir.join("library.json")) {
            Ok(text) => match serde_json::from_str::<Value>(&text) {
                Ok(Value::Object(m)) => {
                    for (k, v) in m {
                        if lib_key_ok(&k) {
                            let t = v.to_string();
                            items.insert(format!("{}{}", LIB, k), (hash(&t), t));
                        }
                    }
                }
                _ => {
                    skip.insert(LIB.to_string());
                }
            },
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Err(_) => {
                skip.insert(LIB.to_string());
            }
        }
    }
    (items, skip)
}

fn lib_key_ok(k: &str) -> bool {
    !k.is_empty() && !LOCAL.contains(&k) && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// A name that syncs: one of the files, or a library part.
fn item_ok(n: &str) -> bool {
    FILES.contains(&n) || n.strip_prefix(LIB).is_some_and(lib_key_ok)
}

fn skipped(skip: &BTreeSet<String>, n: &str) -> bool {
    skip.contains(n) || (n.starts_with(LIB) && skip.contains(LIB))
}

fn stamp(p: &Path) -> Option<(SystemTime, u64)> {
    let m = std::fs::metadata(p).ok()?;
    Some((m.modified().ok()?, m.len()))
}

/// The times and sizes of the files that sync (a change: read them again).
fn stamps(dir: &Path) -> BTreeMap<&'static str, Option<(SystemTime, u64)>> {
    FILES.iter().copied().chain(["library.json"]).map(|f| (f, stamp(&dir.join(f)))).collect()
}

fn modified_ms(dir: &Path, name: &str) -> Option<u64> {
    let f = if name.starts_with(LIB) { "library.json" } else { name };
    let t = std::fs::metadata(dir.join(f)).ok()?.modified().ok()?;
    Some(t.duration_since(UNIX_EPOCH).ok()?.as_millis() as u64)
}

/// A file written whole: a new copy, then renamed over the old (tried a few times: the app may be reading it).
fn write_whole(path: &Path, text: &str) -> Result<(), String> {
    let tmp = path.with_extension("sync.tmp");
    std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
    let mut why = String::new();
    for _ in 0..20 {
        match std::fs::rename(&tmp, path) {
            Ok(()) => return Ok(()),
            Err(e) => {
                why = e.to_string();
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
    let _ = std::fs::remove_file(&tmp);
    Err(why)
}

/// The names of items that came in, for the log.
fn names_of(items: &[(String, String, bool)]) -> String {
    items.iter().map(|x| x.0.as_str()).collect::<Vec<_>>().join(", ")
}

fn log(s: &str) {
    if !cfg!(test) {
        crate::api::log(&format!("sync: {}", s));
    }
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

fn now_s() -> u64 {
    now_ms() / 1000
}

/// What goes over the network, as JSON (each message signed once both hold the key: `Conn`).
#[derive(Serialize, Deserialize)]
#[serde(tag = "t")]
enum Wire {
    /// a round: who calls, and a fresh number
    Hello { id: u64, nonce: u64 },
    /// the answer, proving it holds the key
    Proof { nonce: u64, mac: u64 },
    /// linking, from both sides: who, a fresh number, and whether it's the host (its code typed on the other)
    Join { id: u64, name: String, nonce: u64, host: bool },
    /// the proof of the code, from the one linking
    Answer { mac: u64 },
    /// the host's proof of the code, and the key (hidden with the code)
    Welcome { mac: u64, key: [u64; 2] },
    /// what's on the calling side, and what it couldn't read just now ("library/": all of the library)
    State { items: Vec<Have>, skip: Vec<String> },
    /// what the calling side is to take, what both now agree on, and what to send the text of next time
    Plan { put: Vec<Put>, agreed: Vec<(String, u64)>, want: Vec<String> },
    Ack,
    Busy,
    No { why: String },
}

/// One item on the calling side: its hash (0: not there), the hash both last agreed on (0: never), when it changed
/// there (ms), and its text when it changed (the other side may take it).
#[derive(Serialize, Deserialize, Clone)]
struct Have {
    name: String,
    hash: u64,
    base: u64,
    at: u64,
    text: Option<String>,
}

/// An item to take: its text, and whether to keep the old one first.
#[derive(Serialize, Deserialize, Clone)]
struct Put {
    name: String,
    text: String,
    keep: bool,
}

/// A connection: each message with its length first and, once both hold the key, a signature after (over the side
/// it's from and its number in that direction, so one can't be replayed).
struct Conn {
    s: TcpStream,
    key: Option<[u64; 2]>,
    /// 0 the side that called, 1 the other
    side: u8,
    sent: u64,
    got: u64,
}

impl Conn {
    fn new(s: TcpStream, side: u8) -> Conn {
        let _ = s.set_read_timeout(Some(TIMEOUT));
        let _ = s.set_write_timeout(Some(TIMEOUT));
        let _ = s.set_nodelay(true);
        Conn { s, key: None, side, sent: 0, got: 0 }
    }

    fn send(&mut self, w: &Wire) -> Result<(), String> {
        let body = serde_json::to_vec(w).map_err(|e| e.to_string())?;
        let sig = self.key.map_or(0, |k| mac(k, &[&[self.side], &le(self.sent), &body]));
        self.sent += 1;
        let mut out = Vec::with_capacity(body.len() + 12);
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend_from_slice(&body);
        out.extend_from_slice(&le(sig));
        self.s.write_all(&out).map_err(|e| e.to_string())
    }

    fn recv(&mut self) -> Result<Wire, String> {
        let mut n = [0u8; 4];
        self.s.read_exact(&mut n).map_err(|e| e.to_string())?;
        let n = u32::from_le_bytes(n) as usize;
        if n > MAX {
            return Err("a message too big".to_string());
        }
        let mut body = vec![0u8; n + 8];
        self.s.read_exact(&mut body).map_err(|e| e.to_string())?;
        let mut sig = [0u8; 8];
        sig.copy_from_slice(&body[n..]);
        body.truncate(n);
        if let Some(k) = self.key {
            if u64::from_le_bytes(sig) != mac(k, &[&[1 - self.side], &le(self.got), &body]) {
                return Err("a message not signed with the shared key".to_string());
            }
        }
        self.got += 1;
        serde_json::from_slice(&body).map_err(|e| e.to_string())
    }
}

/// An announcement heard.
struct Hello {
    id: u64,
    port: u16,
    linked: bool,
    dirty: bool,
    reply: bool,
    joining: Option<u64>,
    name: String,
}

fn parse_hello(b: &[u8]) -> Option<Hello> {
    let s = std::str::from_utf8(b).ok()?;
    let mut it = s.splitn(6, '|');
    if it.next()? != "BLSYNC1" {
        return None;
    }
    let id = it.next()?.parse().ok()?;
    let port = it.next()?.parse().ok()?;
    let f = it.next()?;
    let joining = it.next()?.parse::<u64>().ok().filter(|j| *j != 0);
    let name = it.next().unwrap_or("").chars().take(64).collect();
    Some(Hello { id, port, linked: f.contains('L'), dirty: f.contains('D'), reply: f.contains('R'), joining, name })
}

/// Where an announcement goes: everyone on the network (and on this computer's own network, for one with more than
/// one network adapter), this computer itself (another copy of Backline here), and the linked computer where it was
/// last.
fn targets(peer: Option<IpAddr>) -> Vec<SocketAddr> {
    let mut v = vec![SocketAddr::from((Ipv4Addr::LOCALHOST, FIND_PORT))];
    if local_only() {
        return v;
    }
    v.push(SocketAddr::from((Ipv4Addr::BROADCAST, FIND_PORT)));
    // (the address this computer reaches the world from; nothing is sent to find it)
    let own = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).and_then(|s| {
        s.connect((Ipv4Addr::new(192, 0, 2, 1), 9))?;
        s.local_addr()
    });
    if let Ok(SocketAddr::V4(a)) = own {
        let o = a.ip().octets();
        if !a.ip().is_loopback() && !a.ip().is_unspecified() {
            v.push(SocketAddr::from((Ipv4Addr::new(o[0], o[1], o[2], 255), FIND_PORT)));
        }
    }
    if let Some(ip) = peer {
        v.push(SocketAddr::new(ip, FIND_PORT));
    }
    v
}

/// SipHash 2-4 over the parts (each with its length first) under a 128-bit key.
#[allow(deprecated)]
fn mac(key: [u64; 2], parts: &[&[u8]]) -> u64 {
    let mut h = std::hash::SipHasher::new_with_keys(key[0], key[1]);
    for p in parts {
        h.write(&(p.len() as u64).to_le_bytes());
        h.write(p);
    }
    h.finish()
}

/// An item's hash (never 0: 0 is "not there").
fn hash(text: &str) -> u64 {
    mac([0, 0], &[text.as_bytes()]).max(1)
}

/// A number no one can guess: the hasher's keys come from the system, and each hasher made is a new one.
fn random() -> u64 {
    use std::hash::BuildHasher;
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos()));
    h.finish()
}

fn new_code() -> String {
    format!("{:06}", random() % 1_000_000)
}

fn digits(code: &str) -> String {
    code.chars().filter(|c| c.is_ascii_digit()).collect()
}

/// The key a code stands for (its digits only: "482 773" is "482773").
fn code_key(code: &str) -> [u64; 2] {
    let d = digits(code);
    [mac([0x6261_636b, 0x6c69_6e65], &[b"code0", d.as_bytes()]), mac([0x6261_636b, 0x6c69_6e65], &[b"code1", d.as_bytes()])]
}

/// What hides the key while linking (from the code and both fresh numbers).
fn pad(ck: [u64; 2], nj: u64, nh: u64) -> [u64; 2] {
    [mac(ck, &[b"key0", &le(nj), &le(nh)]), mac(ck, &[b"key1", &le(nj), &le(nh)])]
}

/// One connection's key, from the shared key and both fresh numbers (the caller's first).
fn session(key: [u64; 2], a: u64, b: u64) -> [u64; 2] {
    [mac(key, &[b"s0", &le(a), &le(b)]), mac(key, &[b"s1", &le(a), &le(b)])]
}

fn le(n: u64) -> [u8; 8] {
    n.to_le_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("backline_sync_{}_{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn rd(d: &Path, f: &str) -> String {
        std::fs::read_to_string(d.join(f)).unwrap_or_default()
    }

    fn json_of(d: &Path, f: &str) -> Value {
        serde_json::from_str(&rd(d, f)).unwrap()
    }

    /// A computer in a test: its own folder, the app standing in as one that puts library parts into library.json
    /// and saves it, and only its listening thread running (the rounds are called by hand).
    fn computer(dir: &Path, id: u64, name: &str) -> Arc<Engine> {
        computer_as(dir, id, name, false)
    }

    /// The same, as an older Backline: one that doesn't know a diary day's notes (they're gone when it saves).
    fn computer_as(dir: &Path, id: u64, name: &str, old: bool) -> Arc<Engine> {
        let slot: Arc<Mutex<Option<Arc<Engine>>>> = Arc::new(Mutex::new(None));
        let (d, s2) = (dir.to_path_buf(), slot.clone());
        let sink: Sink = Box::new(move |inc| {
            if let Incoming::Library(parts) = inc {
                let p = d.join("library.json");
                let mut v: Value = serde_json::from_str(&std::fs::read_to_string(&p).unwrap_or_default()).unwrap_or(json!({}));
                if let Value::Object(m) = &mut v {
                    for (k, x) in parts {
                        m.insert(k, x);
                    }
                }
                if old {
                    if let Some(Value::Array(days)) = v.get_mut("days") {
                        for d in days.iter_mut().filter_map(|d| d.as_object_mut()) {
                            d.remove("note");
                        }
                    }
                }
                std::fs::write(&p, v.to_string()).unwrap();
                if let Some(e) = lock(&s2).as_ref() {
                    *lock(&e.lib_wait) = None;
                }
            }
        });
        let saved = Saved { on: true, id, code: new_code(), peer: None };
        let e = Arc::new(Engine::new(dir.to_path_buf(), name.to_string(), saved, sink));
        *lock(&slot) = Some(e.clone());
        let l = e.clone();
        std::thread::spawn(move || l.listen(0));
        for _ in 0..200 {
            if e.port.load(Ordering::Relaxed) != 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        e
    }

    #[test]
    fn an_older_backline_neither_loops_nor_erases_notes() {
        let (dn, dold) = (temp("newer"), temp("older"));
        std::fs::write(dn.join("library.json"), json!({"days": [{"date": "2026-11-04", "secs": {"Ear": 60}, "note": "good day"}]}).to_string()).unwrap();
        let n = computer(&dn, 10, "NEWER");
        let o = computer_as(&dold, 20, "OLDER", true);
        type_code(&o, &lock(&n.saved).code.clone());
        n.call_join(at(&o), true).unwrap();
        linked_to(&o);
        // the first round: the older one takes the day, and saves it without the notes
        assert!(o.round(at(&n)).is_ok());
        let days = |d: &Path| json_of(d, "library.json")["days"].clone();
        assert_eq!(days(&dold), json!([{"date": "2026-11-04", "secs": {"Ear": 60}}]));
        // rounds after it move nothing, either way (each side's own form of the day)
        let mtime = |d: &Path| std::fs::metadata(d.join("library.json")).unwrap().modified().unwrap();
        o.poll();
        n.poll();
        let (tn, to) = (mtime(&dn), mtime(&dold));
        for _ in 0..3 {
            std::thread::sleep(Duration::from_millis(20));
            assert!(o.round(at(&n)).is_ok());
            assert!(n.round(at(&o)).is_ok());
        }
        assert_eq!((mtime(&dn), mtime(&dold)), (tn, to), "a round moved something");
        // practice on the older one comes over, and the notes stay
        std::fs::write(dold.join("library.json"), json!({"days": [{"date": "2026-11-04", "secs": {"Ear": 90}}]}).to_string()).unwrap();
        assert!(o.round(at(&n)).is_ok());
        assert_eq!(days(&dn), json!([{"date": "2026-11-04", "secs": {"Ear": 90}, "note": "good day"}]));
        for e in [&n, &o] {
            e.gen.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn at(e: &Engine) -> SocketAddr {
        SocketAddr::from((Ipv4Addr::LOCALHOST, e.port.load(Ordering::Relaxed)))
    }

    /// The other computer, once this one has kept it (the side that called may be linked a moment before the
    /// other side has saved its half).
    fn linked_to(e: &Engine) -> Peer {
        for _ in 0..300 {
            if let Some(p) = e.peer() {
                return p;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("not linked");
    }

    fn type_code(e: &Engine, code: &str) {
        *lock(&e.joining) = Some((digits(code), Instant::now() + LINK_WAIT, random()));
    }

    #[test]
    fn decides_by_who_changed_what() {
        // the same, or one side without it
        assert_eq!(decide("plan_log.json", 5, 5, 1, 1, 0, 0, false, false), Pick::Same);
        assert_eq!(decide("plan_log.json", 5, 0, 5, 5, 0, 0, false, false), Pick::Mine { keep: false });
        assert_eq!(decide("plan_log.json", 0, 5, 5, 5, 0, 0, false, false), Pick::Theirs { keep: false });
        // changed on one side only: that side's
        assert_eq!(decide("plan_log.json", 5, 7, 5, 5, 0, 0, true, true), Pick::Theirs { keep: false });
        assert_eq!(decide("plan_log.json", 7, 5, 5, 5, 0, 0, false, false), Pick::Mine { keep: false });
        // changed on both: put together, the newer first; a plan isn't put together, the newer wins
        assert_eq!(decide("library/days", 7, 8, 5, 5, 20, 10, false, false), Pick::Merge { mine_first: true });
        assert_eq!(decide("library/days", 7, 8, 5, 5, 10, 20, true, true), Pick::Merge { mine_first: false });
        assert_eq!(decide("plan.json", 7, 8, 5, 5, 10, 20, false, false), Pick::Theirs { keep: true });
        assert_eq!(decide("plan.json", 7, 8, 5, 5, 30, 20, false, false), Pick::Mine { keep: true });
        // the first time (or one side lost what it agreed): the host's first
        assert_eq!(decide("plan.json", 7, 8, 0, 0, 0, 99, true, false), Pick::Mine { keep: true });
        assert_eq!(decide("plan.json", 7, 8, 7, 0, 99, 0, false, true), Pick::Theirs { keep: true });
        assert_eq!(decide("library/yt", 7, 8, 0, 0, 0, 0, false, false), Pick::Merge { mine_first: false });
        // neither changed since they agreed, each in its own form: left alone
        assert_eq!(decide("library/days", 7, 8, 7, 8, 0, 0, false, false), Pick::Same);
    }

    #[test]
    fn merging_keeps_both_sides_lists() {
        // ticks: every day's from both
        let a = json!({"2026-11-01": [0, 1], "2026-11-02": [3]});
        let b = json!({"2026-11-01": [1, 2], "2026-11-03": [0]});
        assert_eq!(merge(&a, &b, 0), json!({"2026-11-01": [0, 1, 2], "2026-11-02": [3], "2026-11-03": [0]}));
        // lists by id; one on both is put together, the first one's way
        let a = json!([{"id": 1, "title": "A"}, {"id": 2, "title": "B"}]);
        let b = json!([{"id": 2, "title": "B2", "album": "X"}, {"id": 3, "title": "C"}]);
        assert_eq!(merge(&a, &b, 0), json!([{"id": 1, "title": "A"}, {"id": 2, "title": "B", "album": "X"}, {"id": 3, "title": "C"}]));
        // a playlist on both: its songs from both
        let a = json!([{"name": "Gig", "items": [{"id": 1}], "desc": "mine"}]);
        let b = json!([{"name": "Gig", "items": [{"id": 1}, {"id": 4}], "desc": "theirs"}]);
        assert_eq!(merge(&a, &b, 0), json!([{"name": "Gig", "items": [{"id": 1}, {"id": 4}], "desc": "mine"}]));
        // keepers by their text, the days played from both
        let a = json!([{"text": "lick", "played": ["2026-11-01"], "seen": "2026-11-01"}]);
        let b = json!([{"text": "lick", "played": ["2026-11-02"], "seen": "2026-11-02"}]);
        assert_eq!(merge(&a, &b, 0), json!([{"text": "lick", "played": ["2026-11-01", "2026-11-02"], "seen": "2026-11-01"}]));
        // something that isn't a list or a map: the first one's
        assert_eq!(merge(&json!("Solar"), &json!("Blue Monk"), 0), json!("Solar"));
    }

    #[test]
    fn fields_only_this_side_knows_are_kept() {
        // a day from an older Backline (no notes) keeps this side's notes; its own changes come through
        let mut theirs = json!([{"date": "2026-11-04", "secs": {"Ear": 120}}, {"date": "2026-11-05", "secs": {}}]);
        let ours = json!([{"date": "2026-11-04", "secs": {"Ear": 60}, "note": "good day"}]);
        keep_ours(&mut theirs, &ours);
        assert_eq!(theirs, json!([{"date": "2026-11-04", "secs": {"Ear": 120}, "note": "good day"}, {"date": "2026-11-05", "secs": {}}]));
        // a map's own keys (songs by id) taken out on the other side stay out
        let mut theirs = json!({"5": [1]});
        keep_ours(&mut theirs, &json!({"5": [1], "9": [2]}));
        assert_eq!(theirs, json!({"5": [1]}));
        // a record's new field is kept
        let mut theirs = json!({"mode": 1, "levels": [0]});
        keep_ours(&mut theirs, &json!({"mode": 0, "levels": [0], "top": true}));
        assert_eq!(theirs, json!({"mode": 1, "levels": [0], "top": true}));
    }

    #[test]
    fn songs_and_takes_stay_on_their_computer() {
        let d = temp("local");
        let lib = json!({"files": [{"id": 9}], "folders": ["D:\\Music"], "takes": [], "last": null, "recent": [], "removed_files": [],
            "yt": [{"id": 1}], "days": [], "routine_tune": "Solar"});
        std::fs::write(d.join("library.json"), lib.to_string()).unwrap();
        std::fs::write(d.join("plan_log.json"), "{\"2026-11-01\": [0]}").unwrap();
        std::fs::write(d.join("plan_moves.json"), "{ half written").unwrap();
        let (items, skip) = read_here(&d, false);
        let names: Vec<&str> = items.keys().map(|s| s.as_str()).collect();
        assert_eq!(names, ["library/days", "library/routine_tune", "library/yt", "plan_log.json"]);
        // a file being written is left for the next look, not taken as gone
        assert!(skip.contains("plan_moves.json"));
        assert!(!item_ok("library/files") && !item_ok("../library.json") && !item_ok("library/../x") && item_ok("library/yt"));
    }

    #[test]
    fn library_parts_come_back_the_same_through_the_app() {
        let mut s = crate::store::Store::default();
        s.bpm.insert(-1234567890123, 120);
        s.lines_stars.push("Slonimsky 1".to_string());
        s.routine_tune = "Solar".to_string();
        s.day_mut("2026-11-01").secs.insert("Ear training".to_string(), 600);
        s.yt.push(crate::store::Ext { kind: "yt".to_string(), id: 7, src: "abc".to_string(), dur: 123.456, ..Default::default() });
        let d = temp("store");
        std::fs::write(d.join("library.json"), serde_json::to_string(&s).unwrap()).unwrap();
        let (items, _) = read_here(&d, false);
        // the app's way: the parts put into its library, then saved
        let mut m = match serde_json::to_value(crate::store::Store::default()).unwrap() {
            Value::Object(m) => m,
            _ => unreachable!(),
        };
        for (n, (_, t)) in &items {
            m.insert(n.strip_prefix(LIB).unwrap().to_string(), serde_json::from_str(t).unwrap());
        }
        let back: crate::store::Store = serde_json::from_value(Value::Object(m)).unwrap();
        std::fs::write(d.join("library.json"), serde_json::to_string(&back).unwrap()).unwrap();
        let (again, _) = read_here(&d, false);
        let hashes = |x: &BTreeMap<String, (u64, String)>| x.iter().map(|(n, (h, _))| (n.clone(), *h)).collect::<Vec<_>>();
        assert_eq!(hashes(&items), hashes(&again));
    }

    #[test]
    fn two_computers_link_and_sync() {
        let (da, db) = (temp("pc"), temp("laptop"));
        std::fs::write(da.join("plan.json"), "{\"name\": \"the plan\"}").unwrap();
        std::fs::write(da.join("plan_log.json"), "{\"2026-11-01\": [0, 1]}").unwrap();
        std::fs::write(db.join("plan.json"), "{\"name\": \"an old copy\"}").unwrap();
        std::fs::write(db.join("plan_log.json"), "{\"2026-11-02\": [3]}").unwrap();
        let lib_a = json!({"files": [{"id": 5}], "yt": [{"id": 1, "title": "a"}], "days": [{"date": "2026-11-01", "secs": {"Ear": 60}}], "routine_tune": "Blue Monk"});
        let lib_b = json!({"files": [{"id": 9}], "yt": [{"id": 2, "title": "b"}], "routine_tune": "Solar"});
        std::fs::write(da.join("library.json"), lib_a.to_string()).unwrap();
        std::fs::write(db.join("library.json"), lib_b.to_string()).unwrap();
        let a = computer(&da, 10, "PC");
        let b = computer(&db, 20, "LAPTOP");
        let code = lock(&a.saved).code.clone();

        // a wrong code doesn't link
        type_code(&b, if code == "111111" { "222222" } else { "111111" });
        assert!(b.call_join(at(&a), false).is_err());
        assert!(a.peer().is_none() && b.peer().is_none());
        // the right one does, whichever side calls (here the PC calls the laptop, where the code was typed)
        type_code(&b, &code);
        assert_eq!(a.call_join(at(&b), true).unwrap(), "LAPTOP");
        let (pa, pb) = (linked_to(&a), linked_to(&b));
        assert_eq!((pa.id, pb.id), (20, 10));
        assert_eq!(pa.key, pb.key);
        assert!(pb.host && !pa.host);
        assert_ne!(lock(&a.saved).code, code, "a code works once");

        // the first round (the laptop calls): the laptop takes the PC's things as they are, its own kept in
        // sync_backups; nothing of the laptop's goes to the PC; the songs on each disk stay where they are
        let plan_log_pc = rd(&da, "plan_log.json");
        assert!(b.round(at(&a)).is_ok());
        assert_eq!(rd(&db, "plan.json"), rd(&da, "plan.json"));
        assert_eq!(rd(&db, "plan_log.json"), plan_log_pc);
        assert_eq!(rd(&da, "plan_log.json"), plan_log_pc);
        let (la, lb) = (json_of(&da, "library.json"), json_of(&db, "library.json"));
        assert_eq!(la["files"], json!([{"id": 5}]));
        assert_eq!(lb["files"], json!([{"id": 9}]));
        assert_eq!(la["yt"], json!([{"id": 1, "title": "a"}]));
        assert_eq!(lb["yt"], la["yt"]);
        assert_eq!(lb["routine_tune"], "Blue Monk");
        assert_eq!(lb["days"], la["days"]);
        assert_eq!(std::fs::read_dir(db.join("sync_backups")).unwrap().count(), 4);
        assert!(!da.join("sync_backups").exists(), "nothing on the PC was replaced");
        // (the side that was called settles once the caller's last word is in: wait for it before changing files
        // again, or its last write can land after the next change - a slow machine showed it)
        let settle = || {
            for _ in 0..300 {
                if !a.dirty.load(Ordering::Relaxed) && !b.dirty.load(Ordering::Relaxed) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            assert!(!a.dirty.load(Ordering::Relaxed) && !b.dirty.load(Ordering::Relaxed));
        };
        settle();
        // a round, and the called side's end of it: it takes in what the two agreed a moment after the caller is done,
        // and that must land before a file changes again (else the change reads as agreed: a Mac runner showed it)
        let round = |from: &Arc<Engine>, to: &Arc<Engine>| {
            let before = *lock(&to.ok_at);
            let r = from.round(at(to));
            for _ in 0..300 {
                if *lock(&to.ok_at) != before {
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            r.is_ok()
        };
        // (the first round's called side, the PC, done too)
        for _ in 0..300 {
            if lock(&a.ok_at).is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        // a tick on the PC goes across as it is (the PC calls this time)
        std::fs::write(da.join("plan_log.json"), "{\"2026-11-01\": [0, 1], \"2026-11-02\": [3], \"2026-11-03\": [2]}").unwrap();
        assert!(round(&a, &b));
        assert_eq!(rd(&db, "plan_log.json"), rd(&da, "plan_log.json"));
        settle();
        // an untick on the laptop goes across too (only one side changed)
        std::fs::write(db.join("plan_log.json"), "{\"2026-11-01\": [0], \"2026-11-02\": [3], \"2026-11-03\": [2]}").unwrap();
        assert!(round(&b, &a));
        assert_eq!(json_of(&da, "plan_log.json")["2026-11-01"], json!([0]));
        settle();

        // the plan changed on both: the newer wins, and the other is kept
        std::fs::write(da.join("plan.json"), "{\"name\": \"older\"}").unwrap();
        a.poll();
        std::thread::sleep(Duration::from_millis(20));
        std::fs::write(db.join("plan.json"), "{\"name\": \"newer\"}").unwrap();
        b.poll();
        let kept = std::fs::read_dir(da.join("sync_backups")).map_or(0, |r| r.count());
        assert!(round(&b, &a));
        assert_eq!(rd(&da, "plan.json"), "{\"name\": \"newer\"}");
        assert!(std::fs::read_dir(da.join("sync_backups")).unwrap().count() > kept);
        settle();

        // a diary day added on each while apart: both kept
        let mut x = json_of(&da, "library.json");
        x["days"].as_array_mut().unwrap().push(json!({"date": "2026-11-04", "secs": {"Ear": 30}}));
        std::fs::write(da.join("library.json"), x.to_string()).unwrap();
        a.poll();
        let mut y = json_of(&db, "library.json");
        y["days"].as_array_mut().unwrap().push(json!({"date": "2026-11-05", "secs": {"Lines": 90}}));
        std::fs::write(db.join("library.json"), y.to_string()).unwrap();
        assert!(round(&a, &b));
        settle();
        let days = json_of(&db, "library.json")["days"].clone();
        assert_eq!(days.as_array().unwrap().len(), 3);
        assert_eq!(days, json_of(&da, "library.json")["days"]);

        // nothing changed: the two still agree, and a call from a computer it isn't linked with is turned away
        assert!(round(&b, &a));
        let c = computer(&temp("other"), 30, "OTHER");
        lock(&c.saved).peer = Some(Peer { id: 10, key: [1, 2], ..pb.clone() });
        assert!(matches!(c.round(at(&a)), Err(Fail::Said(_))));
        for e in [&a, &b, &c] {
            e.gen.fetch_add(1, Ordering::SeqCst);
        }
    }
}
