//! What comes back from the work done in the background (`Msg`: pages and songs loaded, pictures, sound files
//! ready, sync, charts found...), taken in once a frame: `drain` (the player and the library) hands the rest to
//! `drain_extra` (practice, files, tunes).

use super::*;
use crate::store::Version;

impl App {
    pub(crate) fn drain(&mut self, ctx: &egui::Context) {
        while let Ok(m) = self.rx.try_recv() {
            match m {
                Msg::Code(c, u) => self.login_code = Some((c, u)),
                Msg::LoggedIn => self.after_login(),
                Msg::NeedLogin(e) => {
                    self.auth = Auth::LoggedOut;
                    self.login_err = if crate::api::no_internet(&e) {
                        "NO INTERNET RIGHT NOW. PRESS USE WITHOUT TIDAL: YOUR FILES, KEPT YOUTUBE AND SOUNDCLOUD SONGS, LOOPS AND ALL THE PRACTICE TOOLS WORK. LOG IN WHEN YOU'RE ONLINE".to_string()
                    } else {
                        e
                    };
                }
                Msg::NoInternet => {
                    let first = !self.net_down;
                    self.net_down = true;
                    self.net_try = Instant::now();
                    self.loading = false;
                    // each failed try waits twice as long before the next (30 s, 1, 2, 4... up to 10 minutes)
                    self.net_wait = if first { 30 } else { (self.net_wait * 2).min(600) };
                    if !self.go_offline {
                        self.ctx.request_repaint_after(Duration::from_secs(self.net_wait + 1));
                    }
                    if first {
                        // your Tidal likes as last seen; the ones kept on this computer play
                        let tracks = crate::api::load_liked();
                        if !tracks.is_empty() && self.lib_page.is_none() {
                            let p = Page {
                                title: "My Library".to_string(),
                                total: tracks.len(),
                                tracks,
                                library: true,
                                ..Default::default()
                            };
                            self.liked.extend(p.tracks.iter().map(|t| t.id));
                            let p = Arc::new(p);
                            self.lib_page = Some(p.clone());
                            self.page = Some(p);
                        }
                        self.set_note(if self.go_offline {
                            "OFFLINE MODE: ONLY WHAT'S ON THIS COMPUTER"
                        } else {
                            "NO INTERNET, SO PLAYING WHAT'S ON THIS COMPUTER. TIDAL COMES BACK BY ITSELF"
                        });
                    }
                }
                Msg::Page(mut p, push) => {
                    // a tour (or recording one): the library holds only the tour's song (your own comes back after)
                    if p.library && (tour::demo_kind().is_some() || self.tour_play.is_some()) {
                        p.tracks = self.demo_track.iter().cloned().collect();
                        p.total = p.tracks.len();
                    }
                    crate::api::log(&format!(
                        "page '{}': {} rows, {} tracks (of {})",
                        p.title,
                        p.rows.len(),
                        p.tracks.len(),
                        p.total
                    ));
                    if p.library {
                        self.liked.extend(p.tracks.iter().map(|t| t.id));
                        self.lib_gen += 1;
                        self.lib_at = Some(Instant::now());
                        if p.total > p.tracks.len() {
                            self.spawn_more_tracks(p.tracks.len(), p.total, self.lib_gen);
                        } else {
                            LIB_LOAD.store(1, Ordering::Relaxed);
                        }
                    }
                    if push {
                        if let Some(old) = self.page.take() {
                            self.back.push(old);
                        }
                        self.fwd.clear();
                    }
                    let library = p.library;
                    self.page = Some(Arc::new(p));
                    self.serial += 1;
                    self.loading = false;
                    // a test copy: an artist's page once the library is in (Pat Metheny, or BACKLINE_ARTIST=<id>)
                    static ARTIST_SHOWN: AtomicBool = AtomicBool::new(false);
                    if library
                        && std::env::var("BACKLINE_OPEN").as_deref() == Ok("artist")
                        && !ARTIST_SHOWN.swap(true, Ordering::Relaxed)
                    {
                        let id = std::env::var("BACKLINE_ARTIST").unwrap_or_else(|_| "14068".to_string());
                        let card = Card {
                            kind: Kind::Artist,
                            id,
                            title: String::new(),
                            subtitle: "Artist".to_string(),
                            image: String::new(),
                        };
                        self.load(true, move |a| a.open(&card));
                    }
                }
                Msg::Media(k) => match k {
                    1 => self.apply(Action::Toggle),
                    2 => self.apply(Action::Next),
                    3 => self.apply(Action::Prev),
                    _ => self.apply(Action::StopBtn),
                },
                Msg::LikeFailed(id, like, e) => {
                    if like {
                        self.liked.remove(&id);
                    } else {
                        self.liked.insert(id);
                    }
                    self.set_err(format!("LIKE FAILED: {}", e));
                }
                Msg::LibraryWhole(p) => {
                    self.lib_looking = false;
                    self.lib_gen += 1;
                    self.liked.extend(p.tracks.iter().map(|t| t.id));
                    LIB_LOAD.store(1, Ordering::Relaxed);
                    let fresh = Arc::new(p);
                    let pages = self.page.iter_mut().chain(self.lib_page.iter_mut()).chain(self.back.iter_mut()).chain(self.fwd.iter_mut());
                    for p in pages.filter(|p| p.library) {
                        *p = fresh.clone();
                    }
                    self.tidal_lists = None;
                }
                Msg::LibraryHead(r) => {
                    self.lib_looking = false;
                    if let Ok((newest, total, rows)) = r {
                        self.library_merge(newest, total, rows);
                    }
                }
                Msg::MoreTracksDone(g, ok) => {
                    if g == self.lib_gen {
                        LIB_LOAD.store(if ok { 1 } else { 2 }, Ordering::Relaxed);
                    }
                }
                Msg::MoreTracks(g, v) => {
                    if g == self.lib_gen {
                        self.liked.extend(v.iter().map(|t| t.id));
                        let mut done = false;
                        // during a tour your songs go to your library as it was, not the tour's page
                        if let Some(p) = self.tour_library() {
                            push_tracks(p, &v);
                            done = true;
                        }
                        // every copy of the library page (the one shown, the one kept, and those back and forward), so
                        // going back to it never shows fewer songs than have come in
                        if !done {
                            let pages = self.page.iter_mut().chain(self.lib_page.iter_mut()).chain(self.back.iter_mut()).chain(self.fwd.iter_mut());
                            for p in pages.filter(|p| p.library) {
                                push_tracks(p, &v);
                            }
                        }
                    }
                }
                Msg::Err(e) => {
                    self.loading = false;
                    self.set_err(e);
                }
                Msg::CardSaved(c, on, r) => match r {
                    Ok(()) => {
                        // the page shows it, and the library's ALBUMS / PLAYLISTS list follows along
                        if let Some(p) = self.page.as_mut().filter(|p| p.card.as_ref().is_some_and(|pc| pc.id == c.id)) {
                            Arc::make_mut(p).saved = Some(on);
                        }
                        let row_name = if c.kind == Kind::Album { "Albums" } else { "Playlists" };
                        if let Some(lp) = self.lib_page.as_mut() {
                            let lp = Arc::make_mut(lp);
                            match lp.rows.iter_mut().find(|r| r.0 == row_name) {
                                Some(row) => {
                                    row.1.retain(|x| x.id != c.id);
                                    if on {
                                        row.1.insert(0, c.clone());
                                    }
                                }
                                None => {
                                    if on {
                                        lp.rows.push((row_name.to_string(), vec![c.clone()]));
                                    }
                                }
                            }
                        }
                        let what = if c.kind == Kind::Album { "ALBUMS" } else { "PLAYLISTS" };
                        self.set_note(&if on {
                            format!("SAVED TO YOUR {}", what)
                        } else {
                            format!("REMOVED FROM YOUR {}", what)
                        });
                    }
                    Err(e) => self.set_err(format!("Couldn't {} on Tidal: {}", if on { "save it" } else { "remove it" }, e)),
                },
                Msg::Followed(id, on, r) => match r {
                    Ok(()) => {
                        // the artist's page shows it, and the library's ARTISTS list follows along
                        let mut card: Option<Card> = None;
                        if let Some(p) = self.page.as_mut().filter(|p| p.artist_id == id) {
                            let p = Arc::make_mut(p);
                            p.following = on;
                            card = Some(Card {
                                kind: Kind::Artist,
                                id: id.clone(),
                                title: p.title.clone(),
                                subtitle: "Artist".to_string(),
                                image: p.image.clone(),
                            });
                        }
                        if let Some(lp) = self.lib_page.as_mut() {
                            let lp = Arc::make_mut(lp);
                            match lp.rows.iter_mut().find(|r| r.0 == "Artists") {
                                Some(row) => {
                                    row.1.retain(|c| c.id != id);
                                    if let (true, Some(c)) = (on, card.clone()) {
                                        row.1.insert(0, c);
                                    }
                                }
                                None => {
                                    if let (true, Some(c)) = (on, card) {
                                        lp.rows.push(("Artists".to_string(), vec![c]));
                                    }
                                }
                            }
                        }
                        self.set_note(if on { "FOLLOWING ON TIDAL" } else { "UNFOLLOWED" });
                    }
                    Err(e) => self.set_err(format!("Couldn't {} on Tidal: {}", if on { "follow" } else { "unfollow" }, e)),
                },
                Msg::Audio(g, id, bytes) => {
                    if g == self.play_gen {
                        let n = bytes.len();
                        crate::api::log(&format!("downloaded {} bytes for track {}", n, id));
                        self.start_wave(id, &bytes);
                        self.start_meta(id, &bytes);
                        self.start_beats(id, &bytes);
                        self.player.send(Cmd::Play(bytes));
                        self.after_play();
                        self.set_kbps(n);
                        self.buffering = false;
                        self.paused = false;
                        self.stopped = false;
                        self.prefetch_next(id);
                    }
                }
                Msg::PlayErr(g, e) => {
                    if g == self.play_gen {
                        self.buffering = false;
                        self.stopped = true;
                        // a Tidal song out of reach (offline, or Tidal down): on to the next song that is on this
                        // computer - your files and the songs kept - rather than stopping
                        let tidal = self.cur_track().is_some_and(|t| self.srcmap.get(&t.id).is_none());
                        let here = |t: &Track| cache::has(t.id) || matches!(self.srcmap.get(&t.id), Some(Src::File(_)));
                        let next = self.cur.and_then(|c| (c + 1..self.queue.len()).find(|k| here(&self.queue[*k])));
                        match next.filter(|_| tidal) {
                            Some(k) => {
                                let name = self.queue[k].title.to_uppercase();
                                self.advance_to(k);
                                self.set_err(format!(
                                    "TIDAL CAN'T BE REACHED. PLAYING {} INSTEAD, WHICH IS ON THIS COMPUTER",
                                    name
                                ));
                            }
                            None if tidal && crate::api::no_internet(&e) => {
                                if !self.net_down && !self.offline {
                                    let _ = self.tx.send(Msg::NoInternet);
                                }
                                self.set_err(
                                    "NO INTERNET, AND THIS SONG ISN'T ON THIS COMPUTER YET. YOUR FILES AND KEPT SONGS (MARKED WITH A TICK) STILL PLAY"
                                        .to_string(),
                                )
                            }
                            None if tidal => self.set_err(format!(
                                "TIDAL CAN'T BE REACHED ({}). SONGS WITH LOOPS OR BOOKMARKS, AND YOUR FILES, STILL PLAY",
                                e
                            )),
                            None => self.set_err(format!("CAN'T PLAY: {}", e)),
                        }
                    }
                }
                Msg::Prefetched(id, b) => self.prefetched = Some((id, b)),
                Msg::Lyrics(id, r) => {
                    if let Some(l) = self.lyrics.as_mut() {
                        if l.id == id {
                            match r {
                                Ok((lines, synced)) => {
                                    l.lines = lines;
                                    l.synced = synced;
                                    l.status = 1;
                                }
                                Err(e) => {
                                    crate::api::log(&format!("lyrics: none for {} ({})", id, e));
                                    l.status = 2;
                                }
                            }
                            self.lyric_scroll = 0.0;
                        }
                    }
                }
                Msg::Img(url, img) => {
                    if let Some(ci) = &img {
                        let half = ci.pixels.len() / 2;
                        let mean = |px: &[Color32]| -> Color32 {
                            let n = px.len().max(1) as u32;
                            let s =
                                px.iter().fold([0u32; 3], |a, c| [a[0] + c.r() as u32, a[1] + c.g() as u32, a[2] + c.b() as u32]);
                            Color32::from_rgb((s[0] / n) as u8, (s[1] / n) as u8, (s[2] / n) as u8)
                        };
                        let (top, bot) = ci.pixels.split_at(half);
                        self.images.avg.insert(url.clone(), [mean(top), mean(bot)]);
                    }
                    // (a skin's screenshot is pixel art: crisp when shown bigger, smooth when smaller)
                    let opts = if url.contains("webampskins.org/screenshots") {
                        egui::TextureOptions { magnification: egui::TextureFilter::Nearest, ..egui::TextureOptions::LINEAR }
                    } else {
                        egui::TextureOptions::LINEAR
                    };
                    let tex = img.map(|ci| ctx.load_texture(&url, ci, opts));
                    self.images.trim();
                    self.images.map.insert(url, (tex, Instant::now()));
                }
                Msg::CardTracks(name, r, next) => match r {
                    Ok(tracks) if !tracks.is_empty() => {
                        let n = tracks.len();
                        if next {
                            // each goes in right after the song playing: last first keeps them in order
                            for t in tracks.into_iter().rev() {
                                self.apply(Action::PlayNext(t));
                            }
                        } else {
                            for t in tracks {
                                self.apply(Action::Enqueue(t));
                            }
                        }
                        self.set_note(&format!(
                            "{} {} SONGS FROM {}",
                            if next { "PLAYING NEXT:" } else { "QUEUED" },
                            n,
                            name.to_uppercase()
                        ));
                    }
                    Ok(_) => self.set_note("NO SONGS IN IT"),
                    Err(e) => self.set_err(format!("COULDN'T OPEN {}: {}", name.to_uppercase(), e)),
                },
                other => self.drain_extra(other),
            }
        }
    }
}

impl App {
    pub(crate) fn drain_extra(&mut self, m: Msg) {
        match m {
            Msg::CmImported(r) => match r {
                Ok(names) => {
                    self.cm_list = None;
                    self.set_note(&format!(
                        "{} TRANSCRIPTION{} IMPORTED",
                        names.len(),
                        if names.len() == 1 { "" } else { "S" }
                    ));
                }
                Err(e) => self.set_err(format!("IMPORT: {}", e)),
            },
            Msg::News(r) => match r {
                Ok(mut n) if !n.is_empty() => {
                    let num = |t: &str| t.rsplit('.').next().and_then(|x| x.parse::<u64>().ok()).unwrap_or(0);
                    let (seen, me) = (self.seen_build, crate::update::build());
                    if seen > 0 && seen < me {
                        let newer: Vec<_> = n.iter().filter(|(t, _)| num(t) > seen).cloned().collect();
                        if !newer.is_empty() {
                            n = newer;
                        }
                    } else {
                        n.truncate(3);
                    }
                    self.news = Some(n);
                }
                Ok(_) => self.set_note("NO RELEASE NOTES FOUND"),
                Err(e) => self.set_err(format!("WHAT'S NEW: {}", e)),
            },
            Msg::Outputs(default, list, bt) => {
                let first = self.sys_output.is_empty() && self.out_list.is_empty();
                let old_list = std::mem::replace(&mut self.out_list, list);
                let old_bt = std::mem::replace(&mut self.out_bt, bt);
                let old_default = std::mem::replace(&mut self.sys_output, default.clone());
                // the first look is where things already play
                if first || default.is_empty() {
                    return;
                }
                // where it was playing, and where it plays now: the output you picked while it is there, else the
                // system's default
                let was = self.out_device.clone().filter(|d| old_list.contains(d)).unwrap_or(old_default);
                let picked = self.out_device.clone().filter(|d| self.out_list.contains(d));
                let now = picked.clone().unwrap_or_else(|| default.clone());
                if now == was {
                    return;
                }
                // Bluetooth headphones or a speaker gone (out of range, switched off): the music pauses rather than
                // carry on out loud; anything else, it keeps playing on the new output
                let bt_gone = !self.out_list.contains(&was) && old_bt.contains(&was);
                if bt_gone && self.cur.is_some() && !self.stopped && !self.paused {
                    self.apply(Action::PauseBtn);
                }
                let keep = self.out_device.clone();
                self.apply(Action::SetDevice(picked));
                self.out_device = keep;
                if bt_gone {
                    self.set_note(&format!("{} DISCONNECTED, SO PLAYBACK PAUSED", was.to_uppercase()));
                } else {
                    self.set_note(&format!("NOW PLAYING THROUGH {}", now.to_uppercase()));
                }
            }
            Msg::ShareSaved(r) => match r {
                Ok(p) => self.set_note(&format!("SAVED: {}. IMPORT IT IN BACKLINE ON THE OTHER COMPUTER", p)),
                Err(e) => self.set_err(format!("EXPORT: {}", e)),
            },
            Msg::ShareIn(Ok(s)) => self.share_view = Some((true, s)),
            Msg::SyncFiles(names) => self.sync_files(names),
            Msg::SyncLibrary(parts) => self.sync_library(parts),
            Msg::PackIn(Ok(p)) => {
                if p.parts.is_empty() && p.share.is_none() {
                    self.set_note("NOTHING NEW IN THAT PACK: YOU HAVE IT ALL ALREADY");
                } else {
                    self.show_prefs = false;
                    self.pack_in = Some(p);
                }
            }
            Msg::PackIn(Err(e)) => self.set_err(format!("PACK: {}", e)),
            Msg::PackSaved(r) => match r {
                Ok(p) => self.set_note(&format!("PACK SAVED: {}. SEND IT, AND DROP IT ON BACKLINE TO OPEN IT", p)),
                Err(e) => self.set_err(format!("PACK: {}", e)),
            },
            Msg::ShareIn(Err(e)) => self.set_err(format!("IMPORT: {}", e)),
            Msg::TidalLists(r) => {
                self.tidal_lists_busy = false;
                match r {
                    Ok(l) => self.tidal_lists = Some(l),
                    Err(e) => {
                        self.tidal_lists = Some(Vec::new());
                        self.set_err(format!("TIDAL PLAYLISTS: {}", e));
                    }
                }
            }
            Msg::TidalListMade(r) => match r {
                Ok(said) => {
                    self.toast(&said);
                    self.tidal_lists = None;
                    self.apply(Action::RefreshLibrary);
                }
                Err(e) => {
                    self.toast = None;
                    self.set_err(format!("TIDAL PLAYLIST: {}", e))
                }
            },
            Msg::TidalListDone(r) => match r {
                Ok(note) => self.toast(&note),
                Err(e) => {
                    self.toast = None;
                    self.set_err(format!("TIDAL PLAYLIST: {}", e))
                }
            },
            Msg::PeerFiles(r) => self.peer_files_in(r),
            Msg::Copied(job, r) => self.copied(job, r),
            Msg::CopyEnd(ok) => self.copy_end(ok),
            Msg::TidalListDupe(uuid, name, t) => {
                self.toast = None;
                self.set_note("");
                self.pl_dupe = Some(crate::new_playlist::PlDupe::Tidal(uuid, name, t));
            }
            Msg::TidalListRenamed(uuid, name, r) => match r {
                Ok(()) => {
                    // the new name everywhere it shows: your playlists, the library's LISTS, the page itself
                    if let Some(l) = self.tidal_lists.as_mut() {
                        for (_, n) in l.iter_mut().filter(|(u, _)| *u == uuid) {
                            *n = name.clone();
                        }
                    }
                    for p in self.page.iter_mut().chain(self.back.iter_mut()).chain(self.fwd.iter_mut()).chain(self.lib_page.iter_mut()) {
                        let here = p.card.as_ref().is_some_and(|c| c.id == uuid);
                        let listed = p.rows.iter().any(|(_, cs)| cs.iter().any(|c| c.id == uuid));
                        if !here && !listed {
                            continue;
                        }
                        let pg = Arc::make_mut(p);
                        if here {
                            pg.title = name.clone();
                            if let Some(c) = pg.card.as_mut() {
                                c.title = name.clone();
                            }
                        }
                        for (_, cs) in pg.rows.iter_mut() {
                            for c in cs.iter_mut().filter(|c| c.id == uuid) {
                                c.title = name.clone();
                            }
                        }
                    }
                    self.serial += 1;
                    self.toast(&format!("Renamed to {} on Tidal", name));
                }
                Err(e) => self.set_err(format!("TIDAL PLAYLIST: {}", e)),
            },
            Msg::TidalListRemoved(uuid, id, nth, r) => match r {
                Ok(()) => {
                    // out of the page, and of the same page kept for going back and forward
                    for p in self.page.iter_mut().chain(self.back.iter_mut()).chain(self.fwd.iter_mut()) {
                        if !p.card.as_ref().is_some_and(|c| c.id == uuid) {
                            continue;
                        }
                        let pg = Arc::make_mut(p);
                        let at: Vec<usize> = pg.tracks.iter().enumerate().filter(|(_, t)| t.id == id).map(|(k, _)| k).collect();
                        if let Some(&k) = at.get(nth).or(at.last()) {
                            pg.tracks.remove(k);
                        }
                        if let Some(n) = pg.subtitle.strip_suffix(" tracks").and_then(|n| n.parse::<usize>().ok()) {
                            pg.subtitle = format!("{} tracks", n.saturating_sub(1));
                        }
                    }
                    self.set_note("REMOVED FROM THIS PLAYLIST");
                }
                Err(e) => self.set_err(format!("TIDAL PLAYLIST: {}", e)),
            },
            Msg::Added(files, folders, lists) => {
                self.files_busy = false;
                let (found, picked_folder) = (files.len(), !folders.is_empty());
                for f in folders {
                    if !self.store.folders.contains(&f) {
                        self.store.folders.push(f);
                    }
                }
                let n = self.add_exts(files);
                self.store_dirty = true;
                // playlist files: each a Backline playlist (in LISTS) of its songs, in order
                if !lists.is_empty() {
                    let mut names = Vec::new();
                    for (name, ids) in lists {
                        let items: Vec<Version> = ids
                            .iter()
                            .filter_map(|id| self.store.files.iter().find(|e| e.id == *id))
                            .map(|e| e.to_version())
                            .collect();
                        if items.is_empty() {
                            continue;
                        }
                        names.push(format!("{} ({} SONGS)", name.to_uppercase(), items.len()));
                        self.store.playlists.push(store::Playlist { name, items, ..Default::default() });
                    }
                    if !names.is_empty() {
                        self.set_note(&format!("PLAYLIST ADDED TO LISTS: {}", names.join(", ")));
                        return;
                    }
                }
                // say what happened: songs added, nothing there, already here, or nothing picked
                let note = if n > 0 {
                    format!("ADDED {} SONG{}", n, if n == 1 { "" } else { "S" })
                } else if found > 0 {
                    "THOSE SONGS ARE ALREADY IN YOUR FILES".to_string()
                } else if picked_folder {
                    "NO MUSIC FILES IN THAT FOLDER".to_string()
                } else {
                    "NOTHING ADDED".to_string()
                };
                self.set_note(&note);
            }
            Msg::Scanned(files) => {
                let n = self.add_exts(files);
                if n > 0 {
                    self.set_note(&format!("FOUND {} NEW FILES", n));
                }
            }
            Msg::YtAdded(r) => {
                self.yt_busy = false;
                match r {
                    Ok(e) => {
                        self.yt_msg = format!("ADDED: {}", e.title);
                        self.yt_in.clear();
                        self.register(&e.to_version());
                        if !self.store.yt.iter().any(|x| x.id == e.id) {
                            self.here.insert(e.id);
                            self.store.yt.insert(0, e);
                            self.store_dirty = true;
                        }
                    }
                    Err(e) => {
                        api::log(&format!("youtube: {}", e));
                        self.yt_msg = format!("ERROR: {}", api::plain(&e));
                    }
                }
            }
            Msg::YtList(url, r) => {
                self.yt_busy = false;
                match r {
                    Ok((name, mut list)) => {
                        // (the songs you took out of it stay out)
                        if let Some(l) = self.store.lists.iter().find(|l| l.url == url) {
                            list.retain(|e| !l.removed.contains(&e.id));
                        }
                        self.yt_msg.clear();
                        self.yt_in.clear();
                        for e in &list {
                            self.register(&e.to_version());
                        }
                        // imported: kept with your playlists, and open on its own page
                        match self.store.lists.iter_mut().find(|l| l.url == url) {
                            Some(l) => l.items = list.clone(),
                            None => self.store.lists.push(store::SavedList {
                                kind: "yt".to_string(),
                                name: name.clone(),
                                url: url.clone(),
                                items: list.clone(),
                                ..Default::default()
                            }),
                        }
                        self.store_dirty = true;
                        self.keep_all(list.iter().map(|e| e.id).collect());
                        self.yt_results = list;
                        self.yt_title = name;
                        self.yt_cur = Some(url);
                    }
                    Err(e) => {
                        api::log(&format!("youtube playlist: {}", e));
                        self.yt_msg = format!("ERROR: {}", api::plain(&e));
                    }
                }
            }
            Msg::ScSets(r) => {
                self.sc_busy = false;
                match r {
                    Ok(list) => {
                        self.sc_msg = format!("{} PLAYLISTS. CLICK ONE TO OPEN IT", list.len());
                        self.sc_sets = list;
                    }
                    Err(e) => {
                        api::log(&format!("soundcloud sets: {}", e));
                        self.sc_msg = format!("ERROR: {}", api::plain(&e));
                    }
                }
            }
            Msg::Sc(r) => {
                self.sc_busy = false;
                match r {
                    Ok(mut list) => {
                        // (a saved playlist's songs you took out stay out)
                        if let Some(l) = self.sc_cur.as_ref().and_then(|u| self.store.lists.iter().find(|l| l.url == *u)) {
                            list.retain(|e| !l.removed.contains(&e.id));
                        }
                        self.sc_msg = format!("{} TRACKS", list.len());
                        for e in &list {
                            self.register(&e.to_version());
                        }
                        // the real titles, covers and lengths: four at a time, shown as they arrive
                        let srcs: Vec<String> = list.iter().take(40).map(|e| e.src.clone()).collect();
                        for w in 0..4 {
                            let mine: Vec<String> = srcs.iter().skip(w).step_by(4).cloned().collect();
                            let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                            std::thread::spawn(move || {
                                for s in mine {
                                    if let Ok(e) = sources::yt_info(&s) {
                                        let _ = tx.send(Msg::ScMeta(e));
                                        ctx.request_repaint();
                                    }
                                }
                            });
                        }
                        // a playlist (a link with several songs): kept with your playlists, on its own page
                        if let Some(u) = self.sc_cur.clone().filter(|_| list.len() > 1) {
                            match self.store.lists.iter_mut().find(|l| l.url == u) {
                                Some(l) => l.items = list.clone(),
                                None => {
                                    let name = self
                                        .sc_sets
                                        .iter()
                                        .find(|(_, x)| *x == u)
                                        .map(|(t, _)| t.clone())
                                        .unwrap_or_else(|| sources::list_name(&u));
                                    self.store.lists.push(store::SavedList {
                                        kind: "sc".to_string(),
                                        name,
                                        url: u,
                                        items: list.clone(),
                                        ..Default::default()
                                    });
                                }
                            }
                            self.store_dirty = true;
                            self.keep_all(list.iter().map(|e| e.id).collect());
                        }
                        self.sc_results = list;
                    }
                    Err(e) => {
                        api::log(&format!("soundcloud: {}", e));
                        self.sc_msg = format!("ERROR: {}", api::plain(&e));
                    }
                }
            }
            Msg::ScMeta(e) => {
                let kept = self.store.lists.iter_mut().flat_map(|l| l.items.iter_mut());
                for slot in self.sc_results.iter_mut().chain(self.store.sc.iter_mut()).chain(kept) {
                    if slot.id == e.id {
                        slot.title = e.title.clone();
                        slot.artist = e.artist.clone();
                        slot.dur = e.dur;
                        slot.cover = e.cover.clone();
                    }
                }
                if self.store.sc.iter().chain(self.store.lists.iter().flat_map(|l| l.items.iter())).any(|x| x.id == e.id) {
                    self.store_dirty = true;
                }
            }
            Msg::YtTool(r) => {
                crate::sources::ytdlp_changed();
                self.yt_busy = false;
                match r {
                    Ok(p) => self.yt_msg = format!("YT-DLP READY: {}", p.display()),
                    Err(e) => {
                        api::log(&format!("yt-dlp download: {}", e));
                        self.yt_msg = format!("ERROR: {}", api::plain(&e));
                    }
                }
            }
            Msg::Wave(id, w) => {
                if self.wave.0 == id {
                    self.wave.1 = w;
                }
            }
            Msg::Notes(id, mask, m) => {
                if self.notes.0 == id && self.notes.1 == mask {
                    self.notes_busy = false;
                    match m {
                        Some(m) => self.notes.2 = Some(Arc::new(m)),
                        None => self.set_err("COULDN'T ANALYZE THIS SONG'S NOTES".to_string()),
                    }
                }
            }
            Msg::WaveFine(id, f) => {
                if self.wave_fine.0 == id {
                    self.wave_fine.1 = Some(f);
                }
            }
            Msg::Exported(r) => {
                self.exporting = false;
                match r {
                    Ok(p) => {
                        let path = std::path::PathBuf::from(p);
                        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                        self.set_note(&format!("SAVED: {}", name));
                        self.export_last = Some(path);
                    }
                    Err(e) if e == "cancelled" => {}
                    Err(e) => self.set_err(format!("EXPORT FAILED: {}", e)),
                }
            }
            Msg::ExportDir(d) => {
                self.export_dir = Some(d);
                self.dirty = true;
            }
            Msg::Stems(id, r) => {
                self.stem_busy = 0;
                match r {
                    Ok(()) => {
                        // remembered, so MY TRACKS > WITH STEMS can name it
                        if let Some(t) = self.queue.iter().find(|t| t.id == id).cloned() {
                            self.remember_song(&t);
                        }
                        self.set_note("STEMS READY. USE THEM IN THE STEMS TAB")
                    }
                    Err(e) if e == "cancelled" => self.set_note("SPLIT CANCELED"),
                    Err(e) => self.set_err(format!("STEMS FAILED: {}", e)),
                }
            }
            Msg::StemTool(r) => {
                self.stem_busy = 0;
                match r {
                    Ok(()) => self.set_note("STEM TOOL READY. NOW SPLIT A TRACK"),
                    Err(e) => self.set_err(format!("STEM TOOL DOWNLOAD FAILED: {}", e)),
                }
            }
            Msg::Lookup(name, info, tracks, found) => {
                self.look_busy = false;
                // YouTube recordings: playable from the list like any other song
                for e in &found {
                    self.register(&e.to_version());
                }
                match info {
                    Ok(w) => {
                        if w.writers.is_empty() {
                            self.set_note(&format!("NO WRITER LISTED FOR {}", w.title.to_uppercase()));
                        }
                        self.fill_details(&name, &w);
                    }
                    Err(e) => self.set_note(&format!("NO WRITER INFO FOUND ({})", e)),
                }
                self.look_for = name;
                self.look_tracks = tracks;
            }
            Msg::TuneDetails(name, r) => {
                if let Ok(w) = r {
                    self.fill_details(&name, &w);
                }
            }
            Msg::RecordFound(key, r) => match r {
                Ok(tracks) if !tracks.is_empty() => {
                    *self.store.listened.entry(key.clone()).or_insert(0) += 1;
                    self.store_dirty = true;
                    self.apply(Action::Play(tracks, 0));
                    self.set_note(&format!("LISTENING: {}", key.to_uppercase()));
                }
                Ok(_) => self.set_note(&format!("{}: NO SONGS ON TIDAL", key.to_uppercase())),
                Err(e) => self.set_note(&e.to_uppercase()),
            },
            Msg::Chart(name, r) => {
                self.chart_busy = false;
                // a recommended tune being looked at: its chart on the lead sheet, not one of your tunes
                if self.find_tune(&name).is_none() && self.preview_tune.as_deref() == Some(name.as_str()) {
                    match r {
                        Ok((chart, _, _)) => {
                            self.prac_chart = Some((name.to_uppercase(), chart));
                            self.chart_sel = None;
                            self.chart_tr = 0;
                            self.open_chart();
                            self.chart_sugg.clear();
                        }
                        Err((e, _)) => self.set_note(&format!("NO CHART FOR {}: {}", name.to_uppercase(), e.to_uppercase())),
                    }
                    return;
                }
                let Some(i) = self.find_tune(&name) else { return };
                match r {
                    Ok((chart, key, composer)) => {
                        let t = &mut self.store.tunes[i];
                        t.chart = chart;
                        if t.key.is_empty() {
                            t.key = key;
                        }
                        if t.written_by.is_empty() && !composer.is_empty() {
                            t.written_by = composer;
                        }
                        self.chart_sugg.clear();
                        self.store_dirty = true;
                        self.set_note(&format!("FOUND THE CHANGES FOR {}", name.to_uppercase()));
                    }
                    Err((e, sugg)) => {
                        self.chart_sugg = sugg;
                        self.set_note(&format!("NO CHART FOR {}: {}", name.to_uppercase(), e.to_uppercase()));
                    }
                }
            }
            Msg::UpdateFound(r) => match r {
                Ok(Some(info)) => {
                    if self.upd.as_ref().is_none_or(|u| u.build < info.build) {
                        self.upd = Some(info.clone());
                        self.upd_state = 1;
                        let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                        std::thread::spawn(move || {
                            let _ = tx.send(Msg::UpdateStaged(crate::update::stage(&info)));
                            ctx.request_repaint();
                        });
                    }
                }
                Ok(None) => {
                    if crate::update::build() == 0 {
                        self.upd_note = "Updates are off: this is a development build (no build number).".to_string();
                    } else {
                        // (PREFERENCES says how long ago, from this)
                        self.upd_at = crate::unix_now();
                        self.upd_note.clear();
                        self.dirty = true;
                    }
                }
                Err(e) => {
                    api::log(&format!("update check: {}", e));
                    self.upd_note = format!("Last check failed: {}", e);
                    self.upd_state = 3;
                }
            },
            Msg::Menu(id) => self.run_named(&id),
            Msg::Credits(r) => {
                if let Some(c) = &mut self.credits {
                    c.1 = Some(r);
                }
            }
            Msg::WaList(r, more) => {
                self.wa_busy = false;
                match r {
                    Ok(list) => {
                        self.wa_end = list.len() < 30;
                        if more {
                            self.wa_list.extend(list);
                        } else {
                            self.wa_list = list;
                        }
                    }
                    Err(e) => self.set_err(format!("SKINS: {}", e)),
                }
            }
            Msg::WaSkin(r) => self.wear_skin(r, true),
            Msg::TagsSaved(r) => match r {
                Ok(name) => self.set_note(&format!("FILE TAGS UPDATED: {}", name)),
                Err(e) => self.set_err(format!("RENAMED IN BACKLINE, BUT COULDN'T UPDATE THE FILE: {}", e)),
            },
            Msg::BandSaved(r) => match r {
                Ok(p) => self.set_note(&format!("SAVED: {}", p)),
                Err(e) => self.set_err(format!("COULD NOT SAVE THE BACKING TRACK: {}", e)),
            },
            Msg::TourSong(r) => match r {
                Ok(path) => self.tour_put_on(path),
                Err(e) => crate::api::log(&format!("tour song: {}", e)),
            },
            Msg::IntroFile(f) => {
                if f.is_some() {
                    self.intro_file = f;
                    self.intro = 2;
                    self.dirty = true;
                    self.apply(Action::Intro(4));
                }
            }
            Msg::FontReady(r) => match r {
                Ok(_) => {
                    // kept on disk now: wear it if it is still the one wanted
                    self.apply_pl_font();
                    self.set_note("FONT READY");
                }
                Err(e) => self.set_err(format!("FONT: {}", e)),
            },
            Msg::Meta(id, found) => {
                crate::meta::set_detected(id, found);
                self.store.meta = crate::meta::saved();
                self.store_dirty = true;
                // the playing song's tempo just heard: the count-off can use it
                if self.bpm == 0 && self.cur_track().is_some_and(|t| t.id == id) {
                    if let Some(b) = crate::meta::get(id).and_then(|m| m.bpm) {
                        self.bpm = b as u32;
                        self.sync_loop();
                    }
                }
            }
            Msg::Tuning(id, r) => {
                self.tuning_busy = false;
                match r {
                    Err(e) => self.set_err(format!("TUNING CHECK FAILED: {}", e)),
                    Ok(Some((c, conf))) if conf >= 0.04 => {
                        self.tuning = Some((id, c, conf));
                        // the record's A, kept with the song (PITCH shows it and matches it to yours)
                        let hz = crate::meta::a4_of(c as f32);
                        crate::meta::set_a4(id, Some((hz * 10.0).round() / 10.0));
                        self.store_dirty = true;
                        let mut how = if c.abs() <= 4 {
                            "IT IS TUNED TO A440".to_string()
                        } else {
                            format!("THIS TRACK IS AT A = {:.1} HZ, {} CENTS {} OF A440", hz, c.abs(), if c > 0 { "SHARP" } else { "FLAT" })
                        };
                        if let Some(k) = crate::meta::get(id).and_then(|i| i.key) {
                            how += &format!(". KEY: {}", crate::meta::key_long(k).to_uppercase());
                        }
                        self.set_note(&how);
                    }
                    _ => {
                        self.tuning = Some((id, 0, -1.0));
                        self.set_note("COULDN'T DETECT THE TUNING: TOO LITTLE PITCHED MUSIC");
                    }
                }
            }
            Msg::UpdateStaged(r) => match r {
                Ok(()) => {
                    self.upd_state = 2;
                    self.set_note("UPDATE READY. CLICK THE BAR AT THE TOP");
                }
                Err(e) => {
                    api::log(&format!("update download: {}", e));
                    self.upd_state = 3;
                }
            },
            Msg::Live(q, r) => {
                self.chart_busy = false;
                if q == self.live_q {
                    match r {
                        Ok(found) => {
                            self.live = Some(found);
                            self.chart_sugg.clear();
                        }
                        Err((_, sugg)) => self.chart_sugg = sugg,
                    }
                }
            }
            Msg::BeatsBg(id, r) => {
                self.beat_known.insert(id, r.clone());
                // FIND BEAT was pressed before it was ready: answer it now
                if self.beat_wait == Some(id) {
                    self.beat_wait = None;
                    self.beat_busy = true;
                    let _ = self.tx.send(Msg::Beats(id, r.clone()));
                }
            }
            Msg::Beats(id, r) => {
                self.beat_busy = false;
                match r {
                    Some((period, phase, grid)) => {
                        self.beat_failed.remove(&id);
                        self.beat = Some((id, period, phase));
                        self.beat_grid = Some((id, grid, period));
                        self.mt_gen += 1;
                        self.set_note(&format!("BEAT FOUND: {:.0} BPM. NOW TURN ON LOCK TO TRACK", 60.0 / period));
                    }
                    None => {
                        self.beat_failed.insert(id);
                        self.mt_gen += 1;
                        self.set_note("NO STEADY BEAT FOUND. USING YOUR OWN TEMPO");
                    }
                }
            }
            _ => {}
        }
    }
}
