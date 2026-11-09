//! Songs from your other computer. Song files never sync by themselves (each computer keeps its own); here you copy
//! the ones you want. FILES > the other computer's name lists the songs in its Backline, asked for when you open it
//! (`peer_files_view`): pick some and COPY HERE, or play one from there. The copying goes on quietly, a pill at the
//! foot of the library saying how far it's got (it can be stopped: `draw_copying`). A song in a synced playlist whose
//! file is on the other computer asks first (`peer_ask_panel`): play it from there (a copy for now, not added to your
//! files) or save a copy here (Music\Backline\From <name>, added to your files, and the playlists play it from then
//! on: `Store::copied`). Either can be made the answer every time.

use crate::credits::panel;
use crate::store;
use crate::sync::{self, Shared};
use crate::{pal, para, retro_btn, retro_btn_w, title_line, Action, App, Msg, RowState, Tip};
use eframe::egui::{self, Align, Pos2, Rect, Sense, Vec2};
use std::collections::{BTreeSet, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// One song to copy: what it is on the other computer, kept here (saved to your files) or just to play now, and the
/// song to play once it's here.
#[derive(Clone)]
pub(crate) struct Job {
    pub file: Shared,
    pub keep: bool,
    pub play: Option<i64>,
}

/// The copying going on: the songs waiting, the one coming (its name, the bytes so far and how many), how many are
/// done of how many, and a stop.
#[derive(Default)]
pub(crate) struct Copying {
    pub queue: Mutex<VecDeque<Job>>,
    pub name: Mutex<String>,
    pub got: AtomicU64,
    pub size: AtomicU64,
    pub done: AtomicUsize,
    pub total: AtomicUsize,
    pub stop: AtomicBool,
    pub running: AtomicBool,
}

/// What the other computer's song list shows here, and what's picked.
#[derive(Default)]
pub(crate) struct PeerView {
    /// the list (None: being asked for), and the paths there already copied here
    pub list: Option<Result<Vec<Shared>, String>>,
    pub busy: bool,
    pub here: HashSet<String>,
    pub sel: BTreeSet<String>,
    /// with this computer's songs too: those songs as last taken, the rows of the list (this computer's or not, and
    /// which), sorted, and what they were made from (made again when that changes)
    pub mine: Vec<store::Ext>,
    pub rows: Vec<(bool, usize)>,
    pub sig: (usize, usize, usize, bool),
}

/// The linked computer's name, as it's shown ("ORCHESTRION" reads "Orchestrion"; a test copy may pretend one:
/// BACKLINE_PEER_NAME).
pub(crate) fn peer_name() -> Option<String> {
    let n = sync::peer_name().or_else(|| std::env::var("BACKLINE_PEER_NAME").ok())?;
    if n.chars().any(|c| c.is_lowercase()) {
        return Some(n);
    }
    let mut cs = n.chars();
    Some(cs.next().map(|c| c.to_string() + &cs.as_str().to_lowercase()).unwrap_or_default())
}

/// A size, as people read it: "820 KB", "4.2 MB", "1.3 GB".
pub(crate) fn size_text(b: u64) -> String {
    let b = b as f64;
    if b >= 1e9 {
        format!("{:.1} GB", b / 1e9)
    } else if b >= 1e6 {
        format!("{:.1} MB", b / 1e6)
    } else {
        format!("{:.0} KB", (b / 1e3).max(1.0))
    }
}

/// Where songs copied from the other computer go: your Music folder, under Backline\From <its name>.
fn copies_dir(peer: &str) -> PathBuf {
    let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).unwrap_or_default();
    let music = if home.is_empty() { crate::api::config_dir() } else { PathBuf::from(home).join("Music") };
    let safe: String = peer.chars().map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' { c } else { '_' }).collect();
    music.join("Backline").join(format!("From {}", safe.trim()))
}

/// Where a song only played from the other computer is put for now (cleared each time Backline starts).
pub(crate) fn for_now_dir() -> PathBuf {
    crate::api::config_dir().join("from_other_computer")
}

/// Where a song lands here: in the copies folder by its own name (a different file of that name already there: a
/// number added), or for now under a name made from where it is there.
fn landing(job: &Job, peer: &str) -> PathBuf {
    let there = Path::new(&job.file.path);
    let name = there.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "song".to_string());
    if !job.keep {
        let ext = there.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_default();
        return for_now_dir().join(format!("{:016x}.{}", path_hash(&job.file.path), ext));
    }
    let dir = copies_dir(peer);
    let first = dir.join(&name);
    let same = |p: &Path| std::fs::metadata(p).is_ok_and(|m| job.file.size > 0 && m.len() == job.file.size);
    if !first.exists() || same(&first) {
        return first;
    }
    let (stem, ext) = (
        there.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        there.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default(),
    );
    (2..1000).map(|k| dir.join(format!("{} ({}){}", stem, k, ext))).find(|p| !p.exists() || same(p)).unwrap_or(first)
}

/// A number for a path (the same each time).
fn path_hash(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

/// Copies the songs waiting, one after another, telling the app as each comes in.
fn run(c: Arc<Copying>, peer: String, tx: std::sync::mpsc::Sender<Msg>, ctx: egui::Context) {
    let mut ok = 0usize;
    loop {
        let job = c.queue.lock().unwrap_or_else(|e| e.into_inner()).pop_front();
        let Some(job) = job else { break };
        if c.stop.load(Ordering::Relaxed) {
            break;
        }
        *c.name.lock().unwrap_or_else(|e| e.into_inner()) = job.file.title.clone();
        c.got.store(0, Ordering::Relaxed);
        c.size.store(job.file.size, Ordering::Relaxed);
        ctx.request_repaint();
        let to = landing(&job, &peer);
        // (already here from an earlier copy: kept as it is)
        let there = job.keep && std::fs::metadata(&to).is_ok_and(|m| job.file.size > 0 && m.len() == job.file.size);
        let r = if there { Ok(to) } else { sync::fetch(&job.file.path, &to, &c.got, &c.size, &c.stop).map(|_| to) };
        ok += r.is_ok() as usize;
        c.done.fetch_add(1, Ordering::Relaxed);
        let _ = tx.send(Msg::Copied(job, r));
        ctx.request_repaint();
    }
    c.running.store(false, Ordering::Relaxed);
    let _ = tx.send(Msg::CopyEnd(ok));
    ctx.request_repaint();
}

impl App {
    /// Copy these songs from the other computer, after any already waiting.
    pub(crate) fn copy_songs(&mut self, jobs: Vec<Job>) {
        let Some(peer) = sync::peer_name() else {
            self.set_err("NOT LINKED WITH ANOTHER COMPUTER (PREFERENCES > LINKED DEVICES)".to_string());
            return;
        };
        if jobs.is_empty() {
            return;
        }
        let c = self.copying.clone();
        if !c.running.load(Ordering::Relaxed) {
            c.stop.store(false, Ordering::Relaxed);
            c.done.store(0, Ordering::Relaxed);
            c.total.store(0, Ordering::Relaxed);
        }
        c.total.fetch_add(jobs.len(), Ordering::Relaxed);
        c.queue.lock().unwrap_or_else(|e| e.into_inner()).extend(jobs);
        if !c.running.swap(true, Ordering::Relaxed) {
            let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
            std::thread::spawn(move || run(c, peer, tx, ctx));
        }
    }

    /// Stop copying: the song coming is dropped, and the ones waiting.
    pub(crate) fn stop_copying(&mut self) {
        self.copying.stop.store(true, Ordering::Relaxed);
        self.copying.queue.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }

    /// A song came in from the other computer (or couldn't).
    pub(crate) fn copied(&mut self, job: Job, r: Result<PathBuf, String>) {
        let path = match r {
            Ok(p) => p,
            Err(e) => {
                if e != "Stopped" {
                    self.set_err(format!("COPYING {}: {}", job.file.title.to_uppercase(), e));
                }
                return;
            }
        };
        if job.keep {
            // into your files under its own names (a file without tags would be named after its folder, "From ..."),
            // remembered as the copy of that song there
            let mut e = crate::sources::ext_for_file(&path);
            if !job.file.title.is_empty() {
                e.title = job.file.title.clone();
                e.artist = job.file.artist.clone();
                e.album = job.file.album.clone();
            }
            self.add_exts(vec![e]);
            self.store.copied.insert(job.file.path.clone(), path.to_string_lossy().into_owned());
            self.peer.here.insert(job.file.path.clone());
            self.store_dirty = true;
            self.serial += 1;
        }
        // the songs anywhere here that name it there now play it from here
        let ids: Vec<i64> = self
            .store
            .playlists
            .iter()
            .flat_map(|p| p.items.iter())
            .chain(self.store.tunes.iter().flat_map(|t| t.versions.iter()))
            .filter(|v| v.kind == "file" && v.src == job.file.path)
            .map(|v| v.id)
            .chain(job.play)
            .collect();
        for id in ids {
            self.srcmap.insert(id, crate::Src::File(path.clone()));
        }
        if let Some(id) = job.play {
            if let Some(k) = self.queue.iter().position(|t| t.id == id) {
                self.play_index(k);
            }
        }
    }

    /// The copying is done: how many came in.
    pub(crate) fn copy_end(&mut self, ok: usize) {
        let total = self.copying.total.load(Ordering::Relaxed);
        let name = peer_name().unwrap_or_default();
        if total > 1 || (ok == 1 && !self.copying_for_now()) {
            if ok > 0 {
                self.toast(&format!("Copied {} song{} from {}", ok, if ok == 1 { "" } else { "s" }, name));
            }
        }
        self.copying.total.store(0, Ordering::Relaxed);
        self.copying.done.store(0, Ordering::Relaxed);
    }

    /// The last copy was only to play now (no message for that: the song just plays).
    fn copying_for_now(&self) -> bool {
        self.copying_last_for_now
    }

    /// A song of yours whose file is on the other computer: play it from there, save a copy, or ask (your choice in
    /// `peer_play`).
    pub(crate) fn song_elsewhere(&mut self, t: &crate::api::Track, there: String) {
        match self.peer_play {
            1 => self.get_from_there(t, there, false),
            2 => self.get_from_there(t, there, true),
            _ => self.peer_ask = Some((t.clone(), there)),
        }
    }

    pub(crate) fn get_from_there(&mut self, t: &crate::api::Track, there: String, keep: bool) {
        self.copying_last_for_now = !keep;
        let file = Shared { path: there, title: t.title.clone(), artist: t.artist.clone(), album: t.album.clone(), dur: t.duration, ..Default::default() };
        self.copy_songs(vec![Job { file, keep, play: Some(t.id) }]);
    }

    /// Keep the song list the other computer may copy from in step with your files.
    pub(crate) fn share_files(&mut self) {
        if self.store.files.len() == self.shared_n {
            return;
        }
        self.shared_n = self.store.files.len();
        // (a song copied from the other computer says where it is there: that computer knows it as its own)
        let came: std::collections::HashMap<&String, &String> = self.store.copied.iter().map(|(there, here)| (here, there)).collect();
        let files = self
            .store
            .files
            .iter()
            .map(|e| Shared {
                path: e.src.clone(),
                title: e.title.clone(),
                artist: e.artist.clone(),
                album: e.album.clone(),
                dur: e.dur,
                size: 0,
                from: came.get(&e.src).map(|s| s.to_string()).unwrap_or_default(),
            })
            .collect();
        sync::share(files);
    }

    /// Ask the other computer for its songs.
    pub(crate) fn load_peer_files(&mut self) {
        if self.peer.busy {
            return;
        }
        self.peer.busy = true;
        let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let _ = tx.send(Msg::PeerFiles(sync::peer_files()));
            ctx.request_repaint();
        });
    }

    /// Its songs came (or why not): which of them are here already.
    pub(crate) fn peer_files_in(&mut self, r: Result<Vec<Shared>, String>) {
        self.peer.busy = false;
        // (the list is made again from what came)
        self.peer.sig = Default::default();
        if let Ok(list) = &r {
            // the copies here named after their folder ("From ...", before copies kept their names): their own names,
            // from the computer they came from
            let mut fixed = false;
            for f in list.iter().filter(|f| !f.title.is_empty()) {
                let Some(local) = self.store.copied.get(&f.path).cloned() else { continue };
                if let Some(e) = self.store.files.iter_mut().find(|e| e.src == local && e.artist.starts_with("From ") && e.artist != f.artist) {
                    e.title = f.title.clone();
                    e.artist = f.artist.clone();
                    e.album = f.album.clone();
                    fixed = true;
                }
            }
            if fixed {
                self.store_dirty = true;
                self.serial += 1;
                // (and the other computer sees the names too)
                self.shared_n = usize::MAX;
            }
            // here already: copied before, one of this computer's own that was copied there, or a song of yours by
            // the same name
            let mine: HashSet<(String, String)> =
                self.store.files.iter().map(|e| (e.title.to_lowercase(), e.artist.to_lowercase())).collect();
            let paths: HashSet<&String> = self.store.files.iter().map(|e| &e.src).collect();
            self.peer.here = list
                .iter()
                .filter(|f| {
                    self.store.copied.contains_key(&f.path)
                        || (!f.from.is_empty() && paths.contains(&f.from))
                        || mine.contains(&(f.title.to_lowercase(), f.artist.to_lowercase()))
                })
                .map(|f| f.path.clone())
                .collect();
            let known: HashSet<&String> = list.iter().map(|f| &f.path).collect();
            self.peer.sel.retain(|p| known.contains(p));
        }
        self.peer.list = Some(r);
    }

    /// FILES with the linked computer's songs (`with_here`: and this computer's, in one list by artist and title, each
    /// saying where it is): this computer's play when clicked, the other's are picked to copy here (or played from
    /// there, right-click).
    pub(crate) fn peer_files_view(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>, name: &str, with_here: bool) {
        if self.peer.list.is_none() {
            self.load_peer_files();
        }
        let list = match self.peer.list.take() {
            None => {
                crate::dim_line(ui, &format!("ASKING {} FOR ITS SONGS...", name.to_uppercase()), 2.0, pal().ink2);
                return;
            }
            Some(Err(e)) => {
                para(ui, &e, pal().red);
                if !retro_btn(ui, "TRY AGAIN", false).clicked() {
                    self.peer.list = Some(Err(e));
                }
                return;
            }
            Some(Ok(l)) => l,
        };
        // the rows: this computer's files (with it), then the other's (those here already left out when this
        // computer's are shown: they're among them), sorted together; made again only when what they're from changes
        let sig = (self.store.files.len(), list.len(), self.peer.here.len(), with_here);
        if self.peer.sig != sig {
            self.peer.sig = sig;
            self.peer.mine = if with_here { self.store.files.clone() } else { Vec::new() };
            let (mine, here) = (&self.peer.mine, &self.peer.here);
            let mut rows: Vec<(bool, usize)> = (0..mine.len())
                .map(|k| (true, k))
                .chain((0..list.len()).filter(|&k| !(with_here && here.contains(&list[k].path))).map(|k| (false, k)))
                .collect();
            if with_here {
                rows.sort_by_cached_key(|&(h, k)| {
                    let (a, t) = if h { (&mine[k].artist, &mine[k].title) } else { (&list[k].artist, &list[k].title) };
                    (a.to_lowercase(), t.to_lowercase())
                });
            }
            self.peer.rows = rows;
        }
        let (mine, rows) = (std::mem::take(&mut self.peer.mine), std::mem::take(&mut self.peer.rows));
        let mut refresh = false;
        let words = |&(here, k): &(bool, usize)| -> [String; 3] {
            if here {
                [mine[k].artist.clone(), mine[k].title.clone(), mine[k].album.clone()]
            } else {
                [list[k].artist.clone(), list[k].title.clone(), list[k].album.clone()]
            }
        };
        let theirs: Vec<usize> = rows.iter().filter(|r| !r.0).map(|r| r.1).collect();
        let total: u64 = theirs.iter().map(|&k| list[k].size).sum();
        let head = if with_here {
            format!("{} songs: {} on this computer, {} on {} ({})", rows.len(), mine.len(), theirs.len(), name, size_text(total))
        } else {
            format!("{} songs on {}, {}", list.len(), name, size_text(total))
        };
        title_line(ui, &head, 2.0, pal().ink2);
        para(ui, "Click a song on the other computer to pick it, then copy the ones you picked here: they go in your Music folder and your files. Right-click one to play it from there.", pal().ink2);
        // the songs, filtered as you type
        let filter = self.lib_filter.trim().to_string();
        let order: Vec<usize> = if filter.is_empty() {
            (0..rows.len()).collect()
        } else {
            crate::filtered(&filter, &rows, |r| words(r).to_vec(), if with_here { 22 } else { 21 })
        };
        let picked: Vec<&Shared> = list.iter().filter(|f| self.peer.sel.contains(&f.path)).collect();
        let picked_size: u64 = picked.iter().map(|f| f.size).sum();
        ui.horizontal_wrapped(|ui| {
            let go = if picked.is_empty() {
                "COPY HERE".to_string()
            } else {
                format!("COPY {} HERE ({})", picked.len(), size_text(picked_size))
            };
            if retro_btn(ui, &go, !picked.is_empty()).tip("Copies the songs you picked to this computer, in the background.").clicked()
                && !picked.is_empty()
            {
                let jobs = picked.iter().map(|f| Job { file: (*f).clone(), keep: true, play: None }).collect();
                self.copying_last_for_now = false;
                self.copy_songs(jobs);
                self.peer.sel.clear();
            }
            let shown: Vec<String> = order
                .iter()
                .map(|&i| rows[i])
                .filter(|&(here, k)| !here && !self.peer.here.contains(&list[k].path))
                .map(|(_, k)| list[k].path.clone())
                .collect();
            if !shown.is_empty() && retro_btn(ui, "PICK ALL SHOWN", false).tip(&format!("Picks every song on {} in the list as it's filtered, except those here already.", name)).clicked() {
                self.peer.sel.extend(shown);
            }
            if !self.peer.sel.is_empty() && retro_btn(ui, "CLEAR", false).clicked() {
                self.peer.sel.clear();
            }
            if retro_btn(ui, "REFRESH", false).tip(&format!("Ask {} for its songs again.", name)).clicked() {
                refresh = true;
            }
        });
        self.filter_box(ui, if with_here { "Filter the songs" } else { "Filter the songs there" });
        if order.is_empty() {
            title_line(ui, if rows.is_empty() { "No songs." } else { "No songs match." }, 2.0, pal().ink2);
        }
        // this computer's songs as shown, to play from the one clicked
        let here_tracks: Vec<crate::api::Track> =
            order.iter().map(|&i| rows[i]).filter(|r| r.0).map(|(_, k)| mine[k].to_version().to_track()).collect();
        let e_of = |k: usize| &mine[k];
        let mut here_n = 0usize;
        let there_tag = name.to_uppercase();
        for (row, &i) in order.iter().enumerate() {
            let (is_here, k) = rows[i];
            if is_here {
                let e = e_of(k);
                let left = || if e.artist.is_empty() { e.title.clone() } else { format!("{} - {}", e.artist, e.title) };
                let right = || "THIS COMPUTER".to_string();
                let at = here_n;
                here_n += 1;
                let Some(r) = crate::list_row(ui, row, None, left, right, RowState::Normal, false, false) else { continue };
                if r.clicked() {
                    acts.push(Action::Play(here_tracks.clone(), at));
                }
                continue;
            }
            let f = &list[k];
            let (here, on) = (self.peer.here.contains(&f.path), self.peer.sel.contains(&f.path));
            let left = || if f.artist.is_empty() { f.title.clone() } else { format!("{} - {}", f.artist, f.title) };
            let right = || if here { "ON THIS COMPUTER".to_string() } else { format!("{}  {}", there_tag, size_text(f.size)) };
            let state = if on { RowState::Playing } else if here { RowState::Played } else { RowState::Normal };
            let Some(r) = crate::list_row(ui, row, None, left, right, state, false, false) else { continue };
            if r.clicked() && !here {
                if on {
                    self.peer.sel.remove(&f.path);
                } else {
                    self.peer.sel.insert(f.path.clone());
                }
            }
            r.context_menu(|ui| {
                if crate::menu_item(ui, &format!("Play from {}", name)) {
                    acts.push(Action::PlayFromThere(f.clone()));
                    ui.close_menu();
                }
                if !here && crate::menu_item(ui, "Copy here") {
                    acts.push(Action::CopyHere(vec![f.clone()]));
                    ui.close_menu();
                }
            });
        }
        // (what was taken out to draw from goes back; REFRESH asks again instead)
        self.peer.mine = mine;
        self.peer.rows = rows;
        if refresh {
            self.peer.sig = Default::default();
        } else {
            self.peer.list = Some(Ok(list));
        }
    }

    /// The question when a song's file is on the other computer: play it from there, or save a copy here (and
    /// whether to do that every time).
    pub(crate) fn peer_ask_panel(&mut self, ctx: &egui::Context) {
        let Some((t, there)) = self.peer_ask.clone() else { return };
        let name = peer_name().unwrap_or_else(|| "your other computer".to_string());
        let mut close = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let mut pick: Option<bool> = None;
        let mut always = self.peer_always;
        panel(ctx, "peer_ask", 470.0, |ui| {
            title_line(ui, &format!("ON {}", name.to_uppercase()), 3.0, pal().ink);
            ui.add_space(4.0);
            para(ui, &format!("\"{}\" is on {}, not this computer.", t.title, name), pal().ink2);
            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                if retro_btn_w(ui, &format!("Play from {}", name), 220.0, true)
                    .tip("Plays it over the network. Nothing is added to your files.")
                    .clicked()
                {
                    pick = Some(false);
                }
                if retro_btn_w(ui, "Save a copy here", 170.0, false)
                    .tip("Copies it into your Music folder and your files. Your playlists play it from here after that.")
                    .clicked()
                {
                    pick = Some(true);
                }
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if crate::check_box(ui, "ALWAYS DO THIS", always)
                    .tip("Don't ask again. You can change it in PREFERENCES > LINKED DEVICES.")
                    .clicked()
                {
                    always = !always;
                }
                if retro_btn_w(ui, "CANCEL", 96.0, false).clicked() {
                    close = true;
                }
            });
        });
        self.peer_always = always;
        if let Some(keep) = pick {
            if always {
                self.peer_play = if keep { 2 } else { 1 };
                self.dirty = true;
            }
            self.peer_ask = None;
            self.get_from_there(&t, there, keep);
        } else if close {
            self.peer_ask = None;
        }
    }

    /// The copying, at the foot of the library while it goes on: an arrow coming down, which song of how many, how
    /// far, and a stop.
    pub(crate) fn draw_copying(&mut self, ui: &egui::Ui, area: Rect) {
        let c = self.copying.clone();
        if !c.running.load(Ordering::Relaxed) {
            return;
        }
        let sc = crate::font::ui_scale();
        let (got, size) = (c.got.load(Ordering::Relaxed), c.size.load(Ordering::Relaxed));
        let (done, total) = (c.done.load(Ordering::Relaxed), c.total.load(Ordering::Relaxed).max(1));
        let name = c.name.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let far = if size > 0 { format!("{}%", got * 100 / size.max(1)) } else { size_text(got) };
        let text = if total > 1 {
            format!("Copying {} of {}: {}  {}", (done + 1).min(total), total, name, far)
        } else {
            format!("Getting {}  {}", name, far)
        };
        let px = 1.6;
        let tw = crate::font::text_w(&text, px).min((area.width() - 110.0 * sc).max(80.0));
        let (w, h) = (tw + 76.0 * sc, 30.0 * sc);
        let r = Rect::from_min_size(Pos2::new(area.min.x + 14.0 * sc, area.max.y - 14.0 * sc - h), Vec2::new(w, h));
        let p = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("lib_copying")));
        p.rect_filled(r.translate(Vec2::new(0.0, 2.0)), h / 2.0, egui::Color32::from_black_alpha(90));
        p.rect_filled(r, h / 2.0, pal().beige);
        p.rect_stroke(r, h / 2.0, egui::Stroke::new(1.0_f32, pal().edge));
        // the arrow, coming down over and over
        let t = ui.input(|i| i.time) as f32;
        let fall = (t * 1.4).fract();
        let ax = r.min.x + 17.0 * sc;
        let ay = r.center().y - 6.0 * sc + fall * 8.0 * sc;
        let col = pal().bar_txt.gamma_multiply(1.0 - fall * 0.6);
        let s = egui::Stroke::new(1.8 * sc, col);
        p.line_segment([Pos2::new(ax, ay - 4.0 * sc), Pos2::new(ax, ay + 4.0 * sc)], s);
        p.line_segment([Pos2::new(ax - 3.5 * sc, ay + 0.5 * sc), Pos2::new(ax, ay + 4.0 * sc)], s);
        p.line_segment([Pos2::new(ax + 3.5 * sc, ay + 0.5 * sc), Pos2::new(ax, ay + 4.0 * sc)], s);
        p.line_segment([Pos2::new(ax - 5.0 * sc, r.center().y + 7.0 * sc), Pos2::new(ax + 5.0 * sc, r.center().y + 7.0 * sc)], egui::Stroke::new(1.4 * sc, pal().ink2));
        crate::ptext_fit(&p, Pos2::new(r.min.x + 32.0 * sc, r.center().y - 1.0), Align::Min, &text, px, tw, pal().ink);
        // how far, as a thin line along the foot
        if size > 0 {
            let f = (got as f32 / size as f32).clamp(0.0, 1.0);
            let bar = Rect::from_min_size(Pos2::new(r.min.x + 32.0 * sc, r.max.y - 6.0 * sc), Vec2::new(tw * f, 2.0 * sc));
            p.rect_filled(bar, 1.0, pal().bar_txt);
        }
        // X stops it
        let x = Rect::from_center_size(Pos2::new(r.max.x - 18.0 * sc, r.center().y), Vec2::splat(20.0 * sc));
        let hit = ui.interact(x, egui::Id::new("lib_copying_stop"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        let xc = if hit.hovered() { pal().ink } else { pal().ink2 };
        let k = 4.0 * sc;
        p.line_segment([x.center() + Vec2::new(-k, -k), x.center() + Vec2::new(k, k)], egui::Stroke::new(1.6 * sc, xc));
        p.line_segment([x.center() + Vec2::new(-k, k), x.center() + Vec2::new(k, -k)], egui::Stroke::new(1.6 * sc, xc));
        if hit.tip("Stop copying").clicked() {
            self.stop_copying();
        }
        ui.ctx().request_repaint();
    }
}

/// A song's version, for the copies map: the path here a synced song's path there plays from.
pub(crate) fn copy_of<'a>(copied: &'a std::collections::BTreeMap<String, String>, v: &store::Version) -> Option<&'a String> {
    (v.kind == "file").then(|| copied.get(&v.src)).flatten()
}
