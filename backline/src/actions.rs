//! What you asked for (`Action`: every click and key that changes something), carried out after the frame is
//! drawn: `apply` (playing, the queue, the library, the windows) hands the rest to `apply_extra` (practice,
//! tunes, the diary, tools).

use super::*;
use crate::extras::*;
use crate::store::{Entry, Section, Version};

impl App {
    pub(crate) fn apply(&mut self, a: Action) {
        // going anywhere in the library leaves the side pages (skins, BACKLINE, storage, the log)
        if matches!(
            a,
            Action::Home | Action::Library | Action::Section(_) | Action::Search(_) | Action::Open(_) | Action::GoTo(..)
        ) {
            self.side = None;
        }
        match a {
            Action::StartLogin => self.start_login(),
            Action::Home => {
                self.sec = Sec::Tidal;
                self.back.clear();
                self.fwd.clear();
                self.load(false, |a| a.home());
            }
            Action::Library => {
                self.sec = Sec::Tidal;
                match self.lib_page.clone() {
                    // already loaded: shown straight away, no waiting (and looked over, quietly, if it's been a while)
                    Some(p) => {
                        self.page = Some(p);
                        self.serial += 1;
                        self.library_look();
                    }
                    None => self.load(false, |a| a.library()),
                }
            }
            Action::ArtPreview(url, title, artist) => {
                if !url.is_empty() {
                    self.art_preview = Some((url, title, artist));
                }
            }
            Action::RefreshLibrary => {
                self.sec = Sec::Tidal;
                self.load(false, |a| a.library());
            }
            Action::Tab(t) => {
                self.lib_tab = t;
                self.serial += 1; // scroll back to the top
                self.library_look();
            }
            Action::Search(q) => {
                if !q.trim().is_empty() {
                    self.sec = Sec::Tidal;
                    self.load(true, move |a| a.search(q.trim()));
                }
            }
            Action::Open(c) => {
                self.sec = Sec::Tidal;
                self.load(true, move |a| a.open(&c))
            }
            Action::BioOpen(id, open) => {
                self.ctx.data_mut(|d| d.insert_temp(egui::Id::new(("bio_open", id)), open));
            }
            Action::SaveCard(c, on) => {
                let (api, tx, ctx) = (self.api.clone(), self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let r = api.save_card(&c, on);
                    let _ = tx.send(Msg::CardSaved(c, on, r));
                    ctx.request_repaint();
                });
            }
            Action::Follow(id, on) => {
                let (api, tx, ctx) = (self.api.clone(), self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let r = api.follow_artist(&id, on);
                    let _ = tx.send(Msg::Followed(id, on, r));
                    ctx.request_repaint();
                });
            }
            Action::GoTo(id, to) => {
                self.sec = Sec::Tidal;
                self.load(true, move |a| a.track_page(id, to))
            }
            Action::PickArtist(list) => self.artist_pick = Some(list),
            Action::Credits(id, label) => {
                self.credits = Some((label, None));
                let (api, tx, ctx) = (self.api.clone(), self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let _ = tx.send(Msg::Credits(api.credits(id)));
                    ctx.request_repaint();
                });
            }
            // one history for the whole library: every tab, page, tune and side page (history.rs)
            Action::Back => self.hist_go(true),
            Action::EditInfo(id, title, artist) => {
                let m = meta::get(id).unwrap_or_default();
                self.edit_kb = (
                    m.key.map_or(String::new(), |k| meta::key_short(k).to_string()),
                    m.bpm.map_or(String::new(), |b| b.to_string()),
                );
                self.edit_extra = match self.store.files.iter().find(|e| e.id == id) {
                    Some(e) => (
                        e.album.clone(),
                        std::path::Path::new(&e.src).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                    ),
                    None => (String::new(), String::new()),
                };
                self.edit_info = Some((id, title, artist));
                self.ed.id = 0;
            }
            Action::SaveInfo(id, title, artist) => self.save_info(id, title, artist),
            Action::PlaySequenceAt(chords, gap, tag) => {
                if !chords.is_empty() {
                    self.player.send(Cmd::StopOnce);
                    // a single-note line is played short and clean (over its backing, if it has one); chords ring
                    // into each other
                    let pcm = if tag == "line" { self.line_pcm(&chords, gap) } else { band::sequence_sound(&chords, gap) };
                    self.player.send(Cmd::Once(pcm, 0.9));
                    self.seq_last = Some(chords.clone());
                    self.seq_play = Some((Instant::now(), gap, tag, chords.len()));
                }
            }
            Action::StopSequence => {
                self.player.send(Cmd::StopOnce);
                self.seq_play = None;
            }
            Action::PlayNotes(notes) => {
                if !notes.is_empty() {
                    self.player.send(Cmd::Once(band::chord_sound(&notes), 0.9));
                }
            }
            Action::ShowLog => {
                self.side = Some(Side::Log);
                self.show_eq = false;
            }
            Action::BacklinePage => {
                self.side = Some(Side::Backline);
            }
            Action::ToggleVinyl => {
                let on = !VINYL.fetch_xor(true, Ordering::Relaxed);
                if on {
                    // a flick: it spins fast and winds down to its normal speed
                    let id = egui::Id::new("vinyl_angle");
                    let ang: f32 = self.ctx.data(|d| d.get_temp::<(f32, f32)>(id)).map_or(0.0, |s| s.0);
                    self.ctx.data_mut(|d| d.insert_temp(id, (ang, 14.0f32)));
                }
                self.dirty = true;
            }
            Action::SetDevice(d) => {
                self.out_device = d.clone();
                self.dirty = true;
                self.player.send(Cmd::Device(d));
                // the song starts again on the new output, where it was (paused stays paused)
                if let (Some(i), Some(t)) = (self.cur, self.cur_track()) {
                    if !self.stopped {
                        let bytes = match self.srcmap.get(&t.id) {
                            Some(Src::File(p)) => std::fs::read(p).ok(),
                            _ => cache::get(t.id),
                        };
                        match bytes {
                            Some(b) => self.player.send(Cmd::Swap(b, self.pos(), self.paused)),
                            None => {
                                let at = self.pos();
                                self.play_index(i);
                                self.pending_seek = Some(at);
                            }
                        }
                    }
                }
                // the metronome and the band start again on the new output
                self.mt_sig = 0;
                self.band_sig = 0;
                self.set_note("SOUND OUTPUT CHANGED");
            }
            Action::WaOpen => {
                self.side = if self.side == Some(Side::Skins) { None } else { Some(Side::Skins) };
                if self.side == Some(Side::Skins) {
                    self.show_eq = false;
                    self.skins_side = if crate::backline_look(self.wa_worn.as_ref()) || self.wa_worn.is_none() { 0 } else { 1 };
                    if self.wa_list.is_empty() && !self.wa_busy {
                        self.apply(Action::WaSearch);
                    }
                }
            }
            Action::WaSearch | Action::WaMore => {
                let more = matches!(a, Action::WaMore);
                if self.wa_busy || (more && self.wa_end) {
                    return;
                }
                self.wa_busy = true;
                if !more {
                    self.wa_list.clear();
                    self.wa_end = false;
                }
                let (q, from) = (self.wa_q.clone(), if more { self.wa_list.len() } else { 0 });
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let _ = tx.send(Msg::WaList(winamp::list(&q, from), more));
                    ctx.request_repaint();
                });
            }
            Action::WaApply(s) => {
                self.wa_loading = Some(s.name.clone());
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let r = winamp::fetch(&s).and_then(|b| winamp::load(&b)).map(|(p, art)| (s, p, art));
                    let _ = tx.send(Msg::WaSkin(r));
                    ctx.request_repaint();
                });
            }
            Action::WaForget(md5) => {
                self.wa_recent.retain(|r| r.0 != md5);
                self.dirty = true;
            }
            Action::WaFav(md5, name) => {
                if self.wa_favs.iter().any(|f| f.0 == md5) {
                    self.wa_favs.retain(|f| f.0 != md5);
                    self.set_note(&format!("REMOVED FROM FAVORITES: {}", name.to_uppercase()));
                } else {
                    self.wa_favs.push((md5, name.clone()));
                    self.set_note(&format!("FAVORITE: {}", name.to_uppercase()));
                }
                self.dirty = true;
            }
            Action::Intro(m) => match m {
                3 => {
                    let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                    std::thread::spawn(move || {
                        let f = rfd::FileDialog::new()
                            .set_title("A short sound for when Backline opens")
                            .add_filter("Audio", &sources::AUDIO_EXT)
                            .pick_file();
                        let _ = tx.send(Msg::IntroFile(f.map(|p| p.to_string_lossy().to_string())));
                        ctx.request_repaint();
                    });
                }
                4 => self.play_intro(),
                m => {
                    self.intro = m.min(2);
                    self.dirty = true;
                }
            },
            Action::Look(n) => {
                self.wear_look(n);
            }
            Action::EditLook => self.edit_look(),
            Action::Balance(b) => {
                self.balance = b.clamp(-100, 100);
                self.player.ctl.set_balance(self.balance);
                self.dirty = true;
            }
            Action::QueueOp(op) => {
                let cur_id = self.cur_track().map(|t| t.id);
                let mut v: Vec<Track> = (*self.queue).clone();
                match op {
                    0 => shuffle_vec(&mut v),
                    1 => v.sort_by_key(|t| (t.artist.to_lowercase(), t.title.to_lowercase())),
                    2 => v.sort_by_key(|t| t.title.to_lowercase()),
                    _ => v.reverse(),
                }
                self.cur = cur_id.and_then(|id| v.iter().position(|t| t.id == id));
                self.queue = Arc::new(v);
                self.orig_queue = None;
                self.set_note(["QUEUE SHUFFLED", "SORTED BY ARTIST", "SORTED BY TITLE", "QUEUE REVERSED"][op.min(3) as usize]);
            }
            Action::PlFont(i) => {
                self.pl_font = i;
                self.dirty = true;
                self.apply_pl_font();
            }
            Action::Forward => self.hist_go(false),
            Action::Play(tracks, i) => {
                if tracks.is_empty() {
                    return;
                }
                self.queue_from = self.list_name(&tracks);
                // a new list: the songs you queued by hand that haven't played yet stay next in line, in their order
                let i = i.min(tracks.len() - 1);
                let kept = self.queued_ahead(tracks[i].id);
                let mut orig = tracks.clone();
                orig.splice(i + 1..i + 1, kept.iter().cloned());
                if self.shuffle {
                    let mut v = shuffled_from(&tracks, i);
                    v.splice(1..1, kept.iter().cloned());
                    self.orig_queue = Some(orig);
                    self.queue = Arc::new(v);
                    self.play_index(0);
                } else {
                    self.orig_queue = None;
                    self.queue = Arc::new(orig);
                    self.play_index(i);
                }
                self.q_manual = kept.len();
            }
            Action::PlayShuffled(tracks) => {
                if tracks.is_empty() {
                    return;
                }
                self.queue_from = self.list_name(&tracks);
                let mut v = tracks.clone();
                shuffle_vec(&mut v);
                // the songs you queued by hand stay next in line, after the first song of the new list
                let kept = self.queued_ahead(v[0].id);
                v.splice(1..1, kept.iter().cloned());
                let mut orig = tracks;
                let at = orig.iter().position(|t| t.id == v[0].id).map_or(0, |p| p + 1);
                orig.splice(at..at, kept.iter().cloned());
                self.shuffle = true;
                self.orig_queue = Some(orig);
                self.queue = Arc::new(v);
                self.play_index(0);
                self.q_manual = kept.len();
            }
            Action::PlayIndex(i) => self.play_index(i),
            Action::PlayNext(mut t) => {
                // its own copy, even when the song is in the queue already
                self.q_next += 1;
                t.qid = self.q_next;
                let at = self.cur.map(|c| c + 1).unwrap_or(0).min(self.queue.len());
                // keep the un-shuffled copy in step (right after the current track)
                let cur_id = self.cur_track().map(|c| c.id);
                if let Some(o) = self.orig_queue.as_mut() {
                    let p = cur_id.and_then(|id| o.iter().position(|x| x.id == id)).map(|p| p + 1).unwrap_or(o.len());
                    o.insert(p.min(o.len()), t.clone());
                }
                self.q_added.insert(t.qid);
                Arc::make_mut(&mut self.queue).insert(at, t);
                if self.cur.is_none() {
                    self.play_index(0);
                } else {
                    self.q_manual += 1;
                    self.refresh_prefetch();
                }
            }
            Action::QueueCard(c, next) => {
                let (api, tx, ctx) = (self.api.clone(), self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let r = api.open(&c).map(|p| p.tracks);
                    let _ = tx.send(Msg::CardTracks(c.title.clone(), r, next));
                    ctx.request_repaint();
                });
            }
            Action::QueueTracks(name, tracks, next) => {
                // (a long list - all of My Tracks - is asked about first: one song is queued from its own menu)
                if tracks.len() > QUEUE_ASK_OVER {
                    self.queue_ask = Some((name, tracks, next));
                } else {
                    let _ = self.tx.send(Msg::CardTracks(name, Ok(tracks), next));
                }
                self.ctx.request_repaint();
            }
            Action::Enqueue(mut t) => {
                self.q_next += 1;
                t.qid = self.q_next;
                self.q_added.insert(t.qid);
                if let Some(o) = self.orig_queue.as_mut() {
                    o.push(t.clone());
                }
                match self.cur {
                    // after the songs you queued, before the rest of the album, playlist or radio
                    Some(c) => {
                        let at = (c + 1 + self.q_manual).min(self.queue.len());
                        Arc::make_mut(&mut self.queue).insert(at, t);
                        self.q_manual += 1;
                        self.refresh_prefetch();
                    }
                    None => {
                        Arc::make_mut(&mut self.queue).push(t);
                        self.play_index(0);
                    }
                }
            }
            Action::Remove(i) => {
                if i >= self.queue.len() || Some(i) == self.cur {
                    return;
                }
                let t = Arc::make_mut(&mut self.queue).remove(i);
                if let Some(c) = self.cur {
                    if i < c {
                        self.cur = Some(c - 1);
                    }
                }
                if let Some(o) = self.orig_queue.as_mut() {
                    if let Some(p) = o.iter().position(|x| x.id == t.id) {
                        o.remove(p);
                    }
                }
                self.refresh_prefetch();
            }
            Action::ClearQueue => {
                self.q_manual = 0;
                if let Some(t) = self.cur_track() {
                    self.queue = Arc::new(vec![t]);
                    self.cur = Some(0);
                } else {
                    self.queue = Arc::new(Vec::new());
                }
                self.orig_queue = if self.shuffle { Some((*self.queue).clone()) } else { None };
                self.prefetched = None;
                self.prefetching = 0;
            }
            Action::PlayBtn => {
                if self.cur.is_none() {
                    if !self.queue.is_empty() {
                        self.play_index(0);
                    }
                } else if self.stopped {
                    if let Some(c) = self.cur {
                        self.play_index(c);
                    }
                } else if self.paused {
                    self.player.send(Cmd::Resume);
                    self.paused = false;
                }
            }
            Action::PauseBtn => {
                if self.cur.is_some() && !self.stopped {
                    if self.paused {
                        self.player.send(Cmd::Resume);
                        self.paused = false;
                    } else {
                        self.player.send(Cmd::Pause);
                        self.paused = true;
                        self.capture_last();
                    }
                }
            }
            Action::StopBtn => {
                self.capture_last();
                self.play_gen += 1;
                self.player.send(Cmd::Stop);
                self.stopped = true;
                self.paused = false;
                self.buffering = false;
            }
            Action::Toggle => {
                if self.cur.is_none() || self.stopped {
                    self.apply(Action::PlayBtn);
                } else {
                    self.apply(Action::PauseBtn);
                }
            }
            Action::Next => {
                if let Some(j) = self.next_index() {
                    self.advance_to(j);
                }
            }
            Action::Prev => {
                if self.pos() > 3.0 {
                    self.player.send(Cmd::Seek(0.0));
                } else if let Some(c) = self.cur {
                    self.play_index(c.saturating_sub(1));
                }
            }
            Action::Seek(t) => self.player.send(Cmd::Seek(t)),
            Action::SeekRel(d) => {
                if self.cur.is_some() && !self.stopped {
                    let dur = self.cur_track().map(|t| t.duration).unwrap_or(0.0);
                    let t = (self.pos() + d).clamp(0.0, (dur - 1.0).max(0.0));
                    self.player.send(Cmd::Seek(t));
                }
            }
            Action::Volume(v) => {
                self.volume = v;
                self.dirty = true;
                self.player.send(Cmd::Volume(v * v));
            }
            Action::Shuffle => {
                let on = !self.shuffle;
                self.set_shuffle(on);
                self.set_note(if on { "SHUFFLE ON, QUEUE REORDERED" } else { "SHUFFLE OFF, BACK TO THE ORIGINAL ORDER" });
            }
            Action::Repeat => {
                self.repeat = match self.repeat {
                    Repeat::Off => Repeat::All,
                    Repeat::All => Repeat::One,
                    Repeat::One => Repeat::Off,
                };
                self.dirty = true;
                self.refresh_prefetch();
            }
            Action::ClearStatus => self.status.clear(),
            Action::CopyLog => {
                let text = crate::api::log_text();
                self.ctx.output_mut(|o| o.copied_text = text);
                self.set_note("LOG COPIED TO CLIPBOARD");
            }
            Action::ToggleLike(t) => {
                if t.id < 0 {
                    // not on Tidal: keep it in the LIKED list on this computer
                    if let Some(i) = self.store.hearts.iter().position(|e| e.id == t.id) {
                        self.store.hearts.remove(i);
                        self.liked.remove(&t.id);
                        self.set_note("REMOVED FROM LIKED");
                    } else {
                        let found = self
                            .store
                            .files
                            .iter()
                            .chain(self.store.yt.iter())
                            .chain(self.store.sc.iter())
                            .chain(self.sc_results.iter())
                            .chain(self.yt_results.iter())
                            .chain(self.store.lists.iter().flat_map(|l| l.items.iter()))
                            .find(|e| e.id == t.id)
                            .cloned()
                            // (anywhere else it shows: made from the song and where it plays from)
                            .or_else(|| match self.srcmap.get(&t.id) {
                                Some(Src::Yt(src)) => Some(store::Ext {
                                    kind: "yt".into(),
                                    id: t.id,
                                    src: src.clone(),
                                    title: t.title.clone(),
                                    artist: t.artist.clone(),
                                    album: t.album.clone(),
                                    dur: t.duration,
                                    cover: t.cover.clone(),
                                }),
                                Some(Src::File(p)) => Some(store::Ext {
                                    kind: "file".into(),
                                    id: t.id,
                                    src: p.to_string_lossy().to_string(),
                                    title: t.title.clone(),
                                    artist: t.artist.clone(),
                                    album: t.album.clone(),
                                    dur: t.duration,
                                    cover: t.cover.clone(),
                                }),
                                None => None,
                            });
                        if let Some(e) = found {
                            self.store.hearts.push(e);
                            self.liked.insert(t.id);
                            self.here.insert(t.id);
                            self.set_note("ADDED TO LIKED (ON THIS PC)");
                        }
                    }
                    self.store_dirty = true;
                    return;
                }
                let like = !self.liked.contains(&t.id);
                if like {
                    self.liked.insert(t.id);
                } else {
                    self.liked.remove(&t.id);
                }
                self.sync_library_like(&t, like);
                let (api, tx, ctx) = (self.api.clone(), self.tx.clone(), self.ctx.clone());
                let id = t.id;
                std::thread::spawn(move || {
                    if let Err(e) = api.set_liked(id, like) {
                        let _ = tx.send(Msg::LikeFailed(id, like, e));
                        ctx.request_repaint();
                    }
                });
                self.set_note(if like { "ADDED TO MY TRACKS" } else { "REMOVED FROM MY TRACKS" });
            }
            Action::QueueNext(i) => {
                if let Some(c) = self.cur {
                    let inside = i > c && i <= c + self.q_manual;
                    self.apply(Action::MoveQueue(i, c + 1));
                    if !inside {
                        self.q_manual += 1;
                    }
                }
            }
            Action::QueueLater(i) => {
                if let Some(c) = self.cur {
                    let q = self.q_manual;
                    // already among the ones you queued: it just goes to the back of them
                    let inside = i > c && i <= c + q;
                    self.apply(Action::MoveQueue(i, c + 1 + q));
                    if !inside {
                        self.q_manual += 1;
                    }
                }
            }
            Action::MoveQueue(from, ins) => {
                let n = self.queue.len();
                if from >= n {
                    return;
                }
                // anywhere, above the song playing too (it keeps playing; its place in the list moves)
                let ins = ins.min(n);
                let to = if ins > from { ins - 1 } else { ins };
                if to == from {
                    return;
                }
                let q = Arc::make_mut(&mut self.queue);
                let t = q.remove(from);
                q.insert(to, t);
                if let Some(c) = self.cur {
                    self.cur = Some(if c == from {
                        to
                    } else {
                        let mut c2 = c;
                        if from < c2 {
                            c2 -= 1;
                        }
                        if to <= c2 {
                            c2 += 1;
                        }
                        c2
                    });
                }
                self.refresh_prefetch();
            }
            Action::Skin => {
                let next = if backline_look(self.wa_worn.as_ref()) && !self.look_custom { SKIN.load(Ordering::Relaxed) + 1 } else { 0 };
                self.wear_look(next);
            }
            Action::Sleep => {
                const STEPS: [u32; 6] = [0, 15, 30, 45, 60, 90];
                let cur = STEPS.iter().position(|m| *m == self.sleep_mins).unwrap_or(0);
                let n = STEPS[(cur + 1) % STEPS.len()];
                self.sleep_mins = n;
                if n == 0 {
                    self.sleep_at = None;
                    self.player.send(Cmd::Volume(self.volume * self.volume));
                    self.set_note("SLEEP TIMER OFF");
                } else {
                    self.sleep_at = Some(Instant::now() + Duration::from_secs(n as u64 * 60));
                    self.set_note(&format!("SLEEP IN {} MIN", n));
                }
            }
            Action::ToggleMini => {
                self.mini = !self.mini;
                // (the system title bar, set again for the full window when it comes back)
                self.deco_now = None;
                if self.mini {
                    // fullscreen first gives way: the mini player is a small window
                    if self.fullscreen {
                        self.apply(Action::ToggleFullscreen);
                    }
                    self.saved_max = self.ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                    if self.saved_max {
                        self.ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
                    }
                    self.saved_size = self.ctx.screen_rect().size();
                    // the full window's equalizer and playlist wait; the mini opens with neither (its EQ and PL
                    // buttons stack the skin's own under the deck)
                    self.mini_saved = (self.show_eq, self.queue_hidden);
                    self.show_eq = false;
                    self.queue_hidden = true;
                    self.mini_stack = (false, false);
                    // the window takes the deck's own shape, at twice its pixels: a Winamp skin's main window is
                    // 275 x 116, Backline's own deck 300 x 138; plus the strip of chips along the top
                    let (deck, extra) = if self.wa_art.is_some() {
                        (Vec2::new(275.0, 116.0), Vec2::ZERO)
                    } else {
                        (Vec2::new(300.0, 138.0), Vec2::new(8.0, 30.0))
                    };
                    // a whole number of screen pixels for each of the deck's pixels (as the skin is drawn), so at any
                    // display scaling the deck fills the window exactly - no dark border round it
                    let ppp = self.ctx.pixels_per_point();
                    let k = (2.0 * ppp).round().max(1.0) / ppp;
                    // a skin's deck is the whole window: no system title bar (its own one drags the window). Taken
                    // off first: taking it off after sizing keeps the outer size and grows the inside round the deck
                    self.ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(self.wa_art.is_none()));
                    self.ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(deck + extra));
                    self.ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(deck * k + extra));
                    self.mini_size_at = Some((Instant::now(), deck * k + extra));
                    // a little player is for listening
                    if self.practice {
                        self.apply(Action::TogglePractice);
                    }
                } else {
                    // the full window's equalizer and playlist as they were
                    (self.show_eq, self.queue_hidden) = self.mini_saved;
                    self.ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(true));
                    self.ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(Vec2::new(980.0, 640.0)));
                    let sz = if self.saved_size.x > 100.0 { self.saved_size } else { Vec2::new(1280.0, 820.0) };
                    // never bigger than the screen (the title bar and the taskbar take some of it)
                    let room = self.ctx.input(|i| i.viewport().monitor_size).unwrap_or(Vec2::new(1920.0, 1080.0))
                        - Vec2::new(16.0, 90.0);
                    self.ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(sz.min(room)));
                    if self.saved_max {
                        self.ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
                    }
                }
            }
            Action::TogglePin => {
                self.pinned = !self.pinned;
                let lvl =
                    if self.pinned { egui::viewport::WindowLevel::AlwaysOnTop } else { egui::viewport::WindowLevel::Normal };
                self.ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(lvl));
            }
            Action::ToggleEq => self.show_eq = !self.show_eq,
            Action::ToggleCacheView => {
                // storage lives in Preferences now
                self.show_prefs = true;
            }
            Action::ClearCache => {
                cache::clear();
                self.cache_stats = cache::stats();
                self.set_note("LOCAL CACHE CLEARED");
            }
            Action::OpenCache => cache::open_folder(),
            Action::ToggleKeep => {
                self.keep_cache = !self.keep_cache;
                self.dirty = true;
                crate::extras::KEEP_ON.store(self.keep_cache, std::sync::atomic::Ordering::Relaxed);
                let n = self.keep_everything();
                self.set_note(&if !self.keep_cache {
                    "NOT SAVING NEW TRACKS".to_string()
                } else if n > 0 {
                    format!("SAVING TRACKS TO DISK. YOUR {} SONGS ARE KEPT IN THE BACKGROUND", n)
                } else {
                    "SAVING TRACKS TO DISK".to_string()
                });
            }
            Action::TogglePractice if PRACTICE => {
                self.practice = !self.practice;
                if self.practice {
                    self.loop_on = self.loop_a.is_some() && self.loop_b.is_some();
                    self.set_note("PRACTICE MODE");
                } else {
                    // just listening: the song at its own speed again
                    self.speed = 100;
                    self.set_note("LISTENING MODE");
                }
                self.sync_loop();
            }
            Action::SetAAt(t) => {
                if self.cur.is_none() || self.stopped {
                    return;
                }
                self.practice = true;
                let t = t.clamp(0.0, self.track_len());
                self.loop_a = Some(t);
                if let Some(b) = self.loop_b {
                    if b <= t + 0.1 {
                        self.loop_b = None;
                    }
                }
                self.loop_on = self.loop_b.is_some();
                self.sync_loop();
                self.set_note(&format!("LOOP A = {}", fmt_t(t)));
            }
            Action::SetBAt(t) => {
                if self.cur.is_none() || self.stopped {
                    return;
                }
                self.practice = true;
                let t = t.clamp(0.0, self.track_len());
                self.loop_b = Some(t);
                match self.loop_a {
                    Some(a) if a + 0.1 < t => {}
                    _ => self.loop_a = Some(0.0),
                }
                self.loop_on = true;
                self.sync_loop();
                if let (Some(a), Some(b)) = (self.loop_a, self.loop_b) {
                    if self.pos() >= b - 0.05 {
                        self.player.send(Cmd::Seek(a));
                    }
                }
                self.set_note(&format!("LOOP B = {}", fmt_t(t)));
            }
            Action::ShiftLoop(on) => {
                let (Some(a), Some(b)) = (self.loop_a, self.loop_b) else {
                    self.set_note("SET LOOP A AND B FIRST");
                    return;
                };
                let len = b - a;
                let na = (if on { a + len } else { a - len }).clamp(0.0, (self.track_len() - len).max(0.0));
                self.loop_a = Some(na);
                self.loop_b = Some(na + len);
                self.loop_on = true;
                self.sync_loop();
                self.player.send(Cmd::Seek(na));
                self.set_note(&format!("LOOP MOVED: {} TO {}", fmt_t(na), fmt_t(na + len)));
            }
            Action::LoopToggle => {
                if self.loop_a.is_none() || self.loop_b.is_none() {
                    self.set_note("SET LOOP A AND B FIRST");
                    return;
                }
                self.practice = true;
                self.loop_on = !self.loop_on;
                if self.loop_on {
                    let (a, b) = (self.loop_a.unwrap_or(0.0), self.loop_b.unwrap_or(0.0));
                    let p = self.pos();
                    if p < a || p >= b {
                        self.player.send(Cmd::Seek(a));
                    }
                }
                self.sync_loop();
            }
            Action::LoopClear => {
                self.loop_a = None;
                self.loop_b = None;
                self.loop_on = false;
                // the current loop is let go: the next A and B make a new one
                self.loop_ident = None;
                self.loop_saved = None;
                self.sync_loop();
                self.set_note("LOOP CLEARED");
            }
            Action::Speed(p) => {
                // playback speed is for everyone, listening or practicing
                self.speed = p.clamp(25, 250);
                self.sync_loop();
            }
            Action::ToggleArt => self.art_view = !self.art_view,
            Action::Visuals => {
                let on = !self.vis_full;
                self.vis_set_full(on, false);
            }
            Action::ToggleSpec => {
                self.show_spec = !self.show_spec;
                self.dirty = true;
            }
            Action::ToggleLyrics => {
                self.show_lyrics = !self.show_lyrics;
                self.dirty = true;
            }
            Action::ToggleFullscreen => {
                self.fullscreen = !self.fullscreen;
                self.fs_by_art = self.fullscreen && self.art_view;
                self.fs_cmd_at = Some(Instant::now());
                self.ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.fullscreen));
            }
            Action::Logout => {
                self.art_view = false;
                self.player.send(Cmd::Stop);
                self.api.logout();
                self.auth = Auth::LoggedOut;
                self.page = None;
                self.lib_page = None;
                self.back.clear();
                self.fwd.clear();
                self.queue = Arc::new(Vec::new());
                self.orig_queue = None;
                self.cur = None;
                self.stopped = true;
                self.shuffle = false;
                self.sec = Sec::Tidal;
            }
            other => self.apply_extra(other),
        }
    }
}

impl App {
    pub(crate) fn apply_extra(&mut self, a: Action) {
        match a {
            Action::TrainerKeys(on) => {
                self.trainer_keys = on;
                self.keys_done = 0;
                self.passes = 0;
                if on && !self.trainer {
                    self.trainer = true;
                    self.practice = true;
                }
                self.set_note(if on {
                    "EVERY FEW PASSES THE LOOP GOES UP A 4TH, THROUGH ALL 12 KEYS"
                } else {
                    "EVERY FEW PASSES THE SPEED GOES UP"
                });
            }
            Action::PreviewTune(name) => {
                if let Some(i) = self.find_tune(&name) {
                    // one of yours: its own chart
                    self.apply(Action::OpenTune(Some(i)));
                    self.open_chart();
                    if self.store.tunes[i].chart.trim().is_empty() {
                        self.apply(Action::FindChart(i, None));
                    }
                    return;
                }
                if self.chart_busy {
                    return;
                }
                self.chart_busy = true;
                self.preview_tune = Some(name.clone());
                self.set_note(&format!("LOOKING UP {}", name.to_uppercase()));
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let r = sources::find_chart(&name);
                    let _ = tx.send(Msg::Chart(name, r));
                    ctx.request_repaint();
                });
            }
            Action::PlayRecord(artist, album) => {
                let key = format!("{} - {}", artist, album);
                self.set_note(&format!("FINDING {}", key.to_uppercase()));
                let (api, tx, ctx) = (self.api.clone(), self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let r = api.find_album(&artist, &album);
                    let _ = tx.send(Msg::RecordFound(key, r));
                    ctx.request_repaint();
                });
            }
            Action::AddRecommended(name) => {
                let i = self.new_tune_named(&name);
                if self.prac_chart.as_ref().is_some_and(|c| c.0.eq_ignore_ascii_case(&name)) {
                    // the chart already on show comes with it
                    if let Some((_, c)) = self.prac_chart.take() {
                        self.store.tunes[i].chart = c;
                    }
                } else if self.store.tunes[i].chart.trim().is_empty() {
                    self.apply(Action::FindChart(i, None));
                }
                self.store_dirty = true;
                self.set_note(&format!("ADDED TO YOUR TUNES: {}", name.to_uppercase()));
            }
            Action::SetTune(kind, name) => {
                // picked for your set: one of your tunes now (learning), its chart looked up
                if name.is_empty() {
                    self.store.routine_set.remove(&kind);
                } else {
                    let i = self.new_tune_named(&name);
                    if self.store.tunes[i].chart.trim().is_empty() {
                        self.apply(Action::FindChart(i, None));
                    }
                    self.store.routine_set.insert(kind.clone(), self.store.tunes[i].name.clone());
                    self.set_note(&format!("{}: {}", kind, name.to_uppercase()));
                }
                self.store_dirty = true;
            }
            Action::RoutineDone(k) => {
                let date = store::today_str();
                let list = self.store.routine_done.entry(date).or_default();
                if list.contains(&k) {
                    list.retain(|x| *x != k);
                } else {
                    list.push(k);
                }
                self.store_dirty = true;
            }
            Action::ShowFile(id) => self.show_file(id),
            Action::FileGone(id) => self.file_gone(id),
            Action::RecToggle => self.rec_toggle(),
            Action::RecLast(keep) => self.rec_last(keep),
            Action::RecPlay(id, dry) => self.take_play(id, dry),
            Action::TakesPanel(open) => {
                self.takes_open = open;
                self.take_ask = None;
                self.ed.id = 0;
            }
            Action::TakeDelete(id) => self.take_delete(id),
            Action::Section(s) => {
                if s.practice() {
                    self.last_practice = s;
                    // coming over from a music page, a practice page turns practice mode on (its panel opens); between
                    // practice pages your Listen / Practice choice stays as you set it
                    let from_music = !self.sec.practice();
                    if crate::PRACTICE && !self.practice && from_music {
                        self.practice = true;
                        self.loop_on = self.loop_a.is_some() && self.loop_b.is_some();
                        self.sync_loop();
                    }
                } else {
                    self.last_music = s;
                }
                self.show_eq = false;
                self.sec = s;
                self.serial += 1;
                if s != Sec::Tunes {
                    self.pick = None;
                }
                if s == Sec::Lists {
                    // saved file / YouTube / SoundCloud items need their sources known before they can play
                    let vs: Vec<Version> = self.store.playlists.iter().flat_map(|p| p.items.iter().cloned()).collect();
                    for v in &vs {
                        self.register(v);
                    }
                }
            }
            Action::AddFiles => self.pick_files(false),
            Action::AddFolder => self.pick_files(true),
            Action::Rescan => {
                self.rescan();
                self.set_note("SCANNING FOR NEW FILES...");
            }
            Action::ForgetFolder(i) => {
                if i < self.store.folders.len() {
                    self.store.folders.remove(i);
                    self.store_dirty = true;
                }
            }
            Action::AddYt => {
                if self.yt_busy {
                    return;
                }
                if sources::is_yt_list(&self.yt_in) {
                    if sources::ytdlp_path().is_none() {
                        self.yt_msg = "PRESS GET YT-DLP FIRST (ONE-TIME SETUP)".to_string();
                        return;
                    }
                    self.yt_busy = true;
                    self.yt_msg = "READING THE PLAYLIST...".to_string();
                    let url = sources::yt_list_url(&self.yt_in);
                    let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                    std::thread::spawn(move || {
                        let r = sources::yt_list(&url);
                        let _ = tx.send(Msg::YtList(url, r));
                        ctx.request_repaint();
                    });
                    return;
                }
                let Some(vid) = sources::yt_video_id(&self.yt_in) else {
                    self.yt_msg = "THAT DOESN'T LOOK LIKE A YOUTUBE LINK".to_string();
                    return;
                };
                if sources::ytdlp_path().is_none() {
                    self.yt_msg = "PRESS GET YT-DLP FIRST (ONE-TIME SETUP)".to_string();
                    return;
                }
                self.yt_busy = true;
                self.yt_msg = "READING THE VIDEO...".to_string();
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let _ = tx.send(Msg::YtAdded(sources::yt_info(&vid)));
                    ctx.request_repaint();
                });
            }
            Action::ScGo => {
                if self.sc_busy {
                    return;
                }
                let q = self.sc_in.trim().to_string();
                if q.is_empty() {
                    return;
                }
                if sources::ytdlp_path().is_none() {
                    self.sc_msg = "PRESS GET YT-DLP FIRST (ONE-TIME SETUP)".to_string();
                    return;
                }
                self.sc_busy = true;
                self.sc_cur = if q.starts_with("http") { Some(q.clone()) } else { None };
                self.sc_msg = "LOOKING...".to_string();
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let r = if q.starts_with("http") { sources::sc_list(&q) } else { sources::sc_search(&q) };
                    let _ = tx.send(Msg::Sc(r));
                    ctx.request_repaint();
                });
            }
            Action::ScSets => {
                if self.sc_busy {
                    return;
                }
                let q = self.sc_in.trim().to_string();
                if q.is_empty() {
                    self.sc_msg = "TYPE A USER NAME FIRST".to_string();
                    return;
                }
                if sources::ytdlp_path().is_none() {
                    self.sc_msg = "PRESS GET YT-DLP FIRST (ONE-TIME SETUP)".to_string();
                    return;
                }
                self.sc_busy = true;
                self.sc_msg = "LOOKING FOR PLAYLISTS...".to_string();
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let _ = tx.send(Msg::ScSets(sources::sc_sets(&q)));
                    ctx.request_repaint();
                });
            }
            Action::CopyName(name) => {
                self.ctx.output_mut(|o| o.copied_text = name.clone());
                self.set_note(&format!("COPIED: {}", name));
            }
            Action::DeleteSong(t) => self.delete_song(&t),
            Action::CopyLink(url) => {
                self.ctx.output_mut(|o| o.copied_text = url.clone());
                self.set_note(&format!("LINK COPIED: {}", url.trim_start_matches("https://")));
            }
            Action::ScOpenSet(url) => {
                self.sc_in = url;
                self.apply(Action::ScGo);
            }
            Action::ScBrowser => {
                use std::sync::atomic::Ordering;
                let n = (sources::SC_BROWSER.load(Ordering::Relaxed) + 1) % 4;
                sources::SC_BROWSER.store(n, Ordering::Relaxed);
                self.dirty = true;
                self.sc_msg = if n == 0 {
                    "BROWSER LOGIN OFF".to_string()
                } else {
                    format!("USING YOUR {} SIGN-IN. CLOSE THAT BROWSER IF IT FAILS.", sources::SC_BROWSERS[n as usize])
                };
            }
            Action::TidalListAdd(uuid, name, t, again) => {
                let (api, tx, ctx) = (self.api.clone(), self.tx.clone(), self.ctx.clone());
                let fresh = uuid.is_empty();
                if fresh {
                    // the new one shows in the list next time it is opened
                    self.tidal_lists = None;
                }
                self.toast(&format!("Adding to {} on Tidal...", name));
                std::thread::spawn(move || {
                    // in it already: asked first (unless that's been asked)
                    if !fresh && !again && api.playlist_has(&uuid, t.id).unwrap_or(false) {
                        let _ = tx.send(Msg::TidalListDupe(uuid, name, t));
                        ctx.request_repaint();
                        return;
                    }
                    let r = if fresh { api.playlist_create(&name, "") } else { Ok(uuid) }
                        .and_then(|u| api.playlist_add(&u, t.id, again))
                        .map(|_| format!("Added to {} on Tidal", name));
                    let _ = tx.send(Msg::TidalListDone(r));
                    ctx.request_repaint();
                });
            }
            Action::CopyHere(files) => {
                self.copying_last_for_now = false;
                let jobs = files.into_iter().map(|file| crate::peer_files::Job { file, keep: true, play: None }).collect();
                self.copy_songs(jobs);
            }
            Action::PlayFromThere(f) => {
                // the song, as one of this computer's would be, to play once it's come over
                let mut t = crate::sources::ext_for_file(std::path::Path::new(&f.path)).to_version().to_track();
                if !f.title.is_empty() {
                    t.title = f.title.clone();
                    t.artist = f.artist.clone();
                }
                t.duration = f.dur;
                self.srcmap.insert(t.id, Src::File(PathBuf::from(&f.path)));
                self.orig_queue = None;
                self.queue = Arc::new(vec![t.clone()]);
                self.cur = None;
                self.q_manual = 0;
                self.get_from_there(&t, f.path.clone(), false);
            }
            Action::TidalListRename(uuid, name) => {
                let d = crate::new_playlist::NewPl { name, tidal: true, tidal_edit: Some(uuid), ..Default::default() };
                self.open_new_playlist(d);
            }
            Action::TidalListRemove(uuid, id, nth) => {
                let (api, tx, ctx) = (self.api.clone(), self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let r = api.playlist_remove(&uuid, id, nth);
                    let _ = tx.send(Msg::TidalListRemoved(uuid, id, nth, r));
                    ctx.request_repaint();
                });
            }
            Action::ShareExport => self.share_view = Some((false, self.store.share())),
            Action::PackMake => {
                let keys: Vec<&'static str> =
                    crate::pack::available().into_iter().map(|(k, _)| k).filter(|k| !self.pack_off.iter().any(|o| o == k)).collect();
                let share = if self.pack_loops { serde_json::to_string_pretty(&self.store.share()).ok() } else { None };
                if keys.is_empty() && share.is_none() {
                    return self.set_note("SELECT AT LEAST ONE ITEM FOR THE PACK");
                }
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let Some(path) = rfd::FileDialog::new()
                        .set_title("Save pack")
                        .set_file_name("Backline pack.backline")
                        .add_filter("Backline pack", &["backline"])
                        .save_file()
                    else {
                        return;
                    };
                    let r = crate::pack::make(&path, &keys, share).map(|_| path.to_string_lossy().to_string());
                    let _ = tx.send(Msg::PackSaved(r));
                    ctx.request_repaint();
                });
            }
            Action::PackOpen(folder) => {
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let picked = if folder {
                        rfd::FileDialog::new().set_title("Pick the pack's folder").pick_folder()
                    } else {
                        rfd::FileDialog::new().set_title("Open a pack").add_filter("Backline pack", &["backline", "zip"]).pick_file()
                    };
                    let Some(path) = picked else { return };
                    let _ = tx.send(Msg::PackIn(crate::pack::open(&path)));
                    ctx.request_repaint();
                });
            }
            Action::PackUnlock => {
                let Some(p) = self.pack_in.take() else { return };
                match crate::pack::unlock(&p.parts) {
                    Ok(done) => {
                        let on = |k: &str| p.parts.iter().any(|x| x.on && x.key == k);
                        // what reads its file once: read it again
                        if on("reading") {
                            self.rd_reload();
                        }
                        if on("pattern") {
                            crate::lines::reload_book();
                        }
                        if on("melodies") {
                            self.cm_list = None;
                        }
                        if !done.is_empty() && !crate::PRACTICE {
                            // kept in Backline's folder: the practice version, opened later, finds them there
                            self.set_note(&format!("SAVED: {}. THEY SHOW UP IN THE PRACTICE VERSION", done.join(", ")));
                        } else if !done.is_empty() {
                            self.set_note(&format!("UNLOCKED: {}", done.join(", ")));
                        }
                        // the loops and playlists: the usual import window, to pick from
                        if let Some(s) = p.share {
                            self.share_view = Some((true, s));
                        }
                    }
                    Err(e) => self.set_err(format!("PACK: {}", e)),
                }
            }
            Action::Bookmark => {
                if let Some(t) = self.cur_track().filter(|t| !self.is_demo(t.id)) {
                    let at = self.pos();
                    self.add_mark(t.id, at);
                }
            }
            Action::News => {
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let _ = tx.send(Msg::News(crate::update::notes(5)));
                    ctx.request_repaint();
                });
            }
            Action::ShareTake => {
                if let Some((true, s)) = self.share_view.take() {
                    let s = self.share_ticked(s);
                    self.take_import(s);
                }
            }
            Action::ShareSave => {
                let Some((false, share)) = self.share_view.take() else { return };
                let share = self.share_ticked(share);
                let text = match serde_json::to_string_pretty(&share) {
                    Ok(t) => t,
                    Err(e) => return self.set_err(format!("EXPORT: {}", e)),
                };
                let (y, m, d, ..) = crate::store::now_local();
                let file = format!("Backline loops and playlists {:04}-{:02}-{:02}.backline", y, m, d);
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let Some(path) = rfd::FileDialog::new()
                        .set_title("Export loops and playlists")
                        .set_file_name(&file)
                        .add_filter("Backline loops and playlists", &["backline"])
                        .save_file()
                    else {
                        return;
                    };
                    let r = std::fs::write(&path, text).map(|_| path.to_string_lossy().to_string()).map_err(|e| e.to_string());
                    let _ = tx.send(Msg::ShareSaved(r));
                    ctx.request_repaint();
                });
            }
            Action::ShareImport => {
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let Some(path) = rfd::FileDialog::new()
                        .set_title("Import loops and playlists")
                        .add_filter("Backline loops and playlists", &["backline", "json"])
                        .pick_file()
                    else {
                        return;
                    };
                    // a pack picked here opens as a pack
                    if crate::pack::is_pack(&path) {
                        let _ = tx.send(Msg::PackIn(crate::pack::open(&path)));
                        ctx.request_repaint();
                        return;
                    }
                    let r = std::fs::read_to_string(&path)
                        .map_err(|e| e.to_string())
                        .and_then(|t| serde_json::from_str::<crate::store::Share>(&t).map_err(|e| e.to_string()))
                        .and_then(|s| {
                            if s.kind == "backline-share" {
                                Ok(s)
                            } else {
                                Err("that isn't a Backline loops and playlists file".to_string())
                            }
                        });
                    let _ = tx.send(Msg::ShareIn(r));
                    ctx.request_repaint();
                });
            }
            Action::PlaylistAdd(i, t, lp, again) => {
                if i < self.store.playlists.len() {
                    let mut v = self.version_of(&t);
                    if let Some((a, b, name)) = lp.clone() {
                        v.loop_ab = Some((a, b));
                        v.loop_name = name;
                    }
                    let name = self.store.playlists[i].name.clone();
                    // the same song (or the same loop of it) in it already: asked first
                    if !again && self.store.playlists[i].items.iter().any(|x| x.same_as(&v)) {
                        self.pl_dupe = Some(crate::new_playlist::PlDupe::Backline(i, name, t, lp));
                    } else {
                        self.store.playlists[i].items.push(v);
                        self.store_dirty = true;
                        self.toast(&format!("Added to {}", name));
                    }
                }
                self.serial += 1;
            }
            Action::PlaylistNew(t) => {
                let d = crate::new_playlist::NewPl {
                    tracks: t.into_iter().collect(),
                    name: self.new_pl.trim().to_string(),
                    ..Default::default()
                };
                self.open_new_playlist(d);
            }
            Action::PlaylistNewFrom(name, tracks, tidal) => {
                self.open_new_playlist(crate::new_playlist::NewPl { name, tracks, tidal, ..Default::default() });
            }
            Action::PlaylistPicker(track, lp) => {
                self.pl_picker = Some(crate::new_playlist::Picker { track, lp, tidal: self.pl_last_tidal });
                self.pl_find.clear();
                self.ed.id = crate::tools::F_PL_FIND;
                self.ed.cur = 0;
                self.ed.anchor = None;
            }
            Action::PlaylistEdit(i) => {
                if let Some(pl) = self.store.playlists.get(i) {
                    let d = crate::new_playlist::NewPl {
                        name: pl.name.clone(),
                        desc: pl.desc.clone(),
                        art: pl.art.clone(),
                        edit: Some(i),
                        ..Default::default()
                    };
                    self.open_new_playlist(d);
                }
            }
            Action::PlaylistMake(d) => {
                // (a Tidal playlist keeps Tidal's own cover; a cover already kept is not copied again)
                let kept = d.edit.and_then(|i| self.store.playlists.get(i)).is_some_and(|p| p.art == d.art);
                let art = if d.art.is_empty() || d.tidal {
                    String::new()
                } else if kept {
                    d.art.clone()
                } else {
                    crate::new_playlist::keep_art(&d.art).unwrap_or_default()
                };
                if let Some(pl) = d.edit.and_then(|i| self.store.playlists.get_mut(i)) {
                    let was = pl.name.clone();
                    if !d.name.trim().is_empty() {
                        pl.name = d.name.trim().to_string();
                    }
                    pl.desc = d.desc.trim().to_string();
                    pl.art = art;
                    let now = pl.name.clone();
                    self.store_dirty = true;
                    self.serial += 1;
                    if now != was {
                        self.toast(&format!("Renamed to {}", now));
                    }
                    return;
                }
                // one of your Tidal playlists, renamed
                if let Some(uuid) = d.tidal_edit.clone() {
                    let name = d.name.trim().to_string();
                    let (api, tx, ctx) = (self.api.clone(), self.tx.clone(), self.ctx.clone());
                    std::thread::spawn(move || {
                        let r = api.playlist_rename(&uuid, &name);
                        let _ = tx.send(Msg::TidalListRenamed(uuid, name, r));
                        ctx.request_repaint();
                    });
                    return;
                }
                let count = d.tracks.len();
                let songs = if count > 1 { format!(" with {} songs", count) } else { String::new() };
                if d.tidal {
                    let name = d.name.trim().to_string();
                    let desc = d.desc.trim().to_string();
                    let ids: Vec<i64> = d.tracks.iter().map(|t| t.id).collect();
                    let (api, tx, ctx) = (self.api.clone(), self.tx.clone(), self.ctx.clone());
                    self.tidal_lists = None;
                    self.toast(&format!("Making {} on Tidal...", name));
                    std::thread::spawn(move || {
                        let r = api.playlist_create(&name, &desc).and_then(|u| {
                            if !ids.is_empty() {
                                api.playlist_add_all(&u, &ids, false)?;
                            }
                            Ok(u)
                        });
                        let _ = tx.send(Msg::TidalListMade(r.map(|_| format!("Made {} on Tidal{}", name, songs))));
                        ctx.request_repaint();
                    });
                } else {
                    let typed = d.name.trim().to_string();
                    let name = if typed.is_empty() { format!("Playlist {}", self.store.playlists.len() + 1) } else { typed };
                    let mut items: Vec<store::Version> = d.tracks.iter().map(|t| self.version_of(t)).collect();
                    if let (Some((a, b, lname)), Some(v)) = (d.loop_ab, items.first_mut()) {
                        v.loop_ab = Some((a, b));
                        v.loop_name = lname;
                    }
                    self.store.playlists.push(store::Playlist { name: name.clone(), items, art, desc: d.desc.trim().to_string() });
                    // made from the LISTS page: open it there; from anywhere else, stay where you are
                    if self.sec == Sec::Lists {
                        self.pl_open = Some(self.store.playlists.len() - 1);
                    }
                    self.toast(&format!("Made {} in Backline{}", name, songs));
                    self.store_dirty = true;
                    self.serial += 1;
                }
                self.new_pl.clear();
            }
            Action::PlaylistFromQueue => {
                if self.queue.is_empty() {
                    self.toast("The queue is empty");
                    return;
                }
                // named after what it was started from (a radio, a mix), or what was typed; the name can be changed
                let typed = self.new_pl.trim().to_string();
                let name = if typed.is_empty() { self.queue_from.clone() } else { typed.clone() };
                let tracks = self.queue.to_vec();
                // songs you added by hand on top of a list you were playing (one liked song started all of My
                // Tracks): asked which you meant, just yours or the whole queue
                let mine: Vec<Track> =
                    tracks.iter().filter(|t| t.qid != 0 && self.q_added.contains(&t.qid)).cloned().collect();
                if !mine.is_empty() && mine.len() < tracks.len() {
                    self.queue_save = Some(crate::new_playlist::QueueSave { typed, from: name, mine, all: tracks });
                    return;
                }
                self.apply(Action::PlaylistNewFrom(name, tracks, false));
            }
            Action::PlaylistRemove(pi, ti) => {
                if let Some(p) = self.store.playlists.get_mut(pi) {
                    if ti < p.items.len() {
                        p.items.remove(ti);
                        self.store_dirty = true;
                    }
                }
            }
            Action::PlaylistMove(pi, ti, d) => {
                if let Some(p) = self.store.playlists.get_mut(pi) {
                    let to = ti as i32 + d;
                    if to >= 0 && (to as usize) < p.items.len() {
                        p.items.swap(ti, to as usize);
                        self.store_dirty = true;
                    }
                }
            }
            Action::PlaylistDelete(pi) => {
                if pi < self.store.playlists.len() {
                    self.store.playlists.remove(pi);
                    self.pl_open = None;
                    self.store_dirty = true;
                }
            }
            Action::PlaylistOpen(i) => {
                self.pl_open = i;
                self.serial += 1;
            }
            Action::FindTuning => {
                let Some(t) = self.cur_track() else {
                    self.set_note("PLAY A TRACK FIRST");
                    return;
                };
                if self.tuning_busy {
                    return;
                }
                // the file, the stored copy, or (not stored: storing tracks is off, or it never finished) the track
                // fetched again to listen to
                let src = self.srcmap.get(&t.id).cloned();
                let (lossless, api) = (self.prefer_lossless, self.api.clone());
                self.tuning_busy = true;
                self.set_note("CHECKING THE TUNING...");
                let (tx, ctx, id) = (self.tx.clone(), self.ctx.clone(), t.id);
                // (the song as the player has it, not fetched again)
                let have = self.playing_bytes(id);
                std::thread::spawn(move || {
                    let bytes = match have {
                        Some(b) => Ok(b.to_vec()),
                        None => crate::fetch_audio(&api, id, lossless, false, src),
                    };
                    let r = bytes.map(crate::tuning::detect);
                    let _ = tx.send(Msg::Tuning(id, r));
                    ctx.request_repaint();
                });
            }
            Action::SetMode(m) => self.set_mode(m),
            Action::QuitApp => {
                self.flush_practice();
                self.capture_last();
                self.save_settings();
                self.store.save();
                std::process::exit(0);
            }
            Action::ToggleHelp => {
                self.show_help = !self.show_help;
                if self.show_help {
                    self.show_prefs = false;
                }
            }
            Action::TogglePrefs => {
                self.show_prefs = !self.show_prefs;
                self.rebinding = None;
                if self.show_prefs {
                    self.show_help = false;
                }
            }
            Action::ApplyUpdate => {
                self.flush_practice();
                // the queue and the song come back after the restart
                self.capture_last();
                self.save_settings();
                self.store.save();
                match crate::update::apply() {
                    Ok(()) => std::process::exit(0),
                    Err(e) => {
                        api::log(&format!("update install: {}", e));
                        self.set_note("UPDATE FAILED. IT WILL TRY AGAIN");
                        self.upd_state = 3;
                    }
                }
            }
            Action::RemoveFromList(url, id) => {
                if let Some(l) = self.store.lists.iter_mut().find(|l| l.url == url) {
                    l.items.retain(|e| e.id != id);
                    if !l.removed.contains(&id) {
                        l.removed.push(id);
                    }
                    self.store_dirty = true;
                }
                self.yt_results.retain(|e| e.id != id);
                self.sc_results.retain(|e| e.id != id);
                // (nowhere else now: its saved copy goes too)
                let gone = self.drop_unused_copies(&[id]);
                self.set_note(if gone > 0 { "REMOVED FROM THIS PLAYLIST, AND ITS SAVED COPY DELETED" } else { "REMOVED FROM THIS PLAYLIST" });
            }
            Action::RemoveList(i) => {
                if i < self.store.lists.len() {
                    let l = self.store.lists.remove(i);
                    self.store_dirty = true;
                    // its songs' saved copies, those nowhere else
                    let ids: Vec<i64> = l.items.iter().map(|e| e.id).collect();
                    self.drop_unused_copies(&ids);
                }
            }
            Action::YtKeep(id) => {
                if let Some(e) = self.yt_results.iter().find(|e| e.id == id).cloned() {
                    if !self.store.yt.iter().any(|x| x.id == id) {
                        self.register(&e.to_version());
                        self.here.insert(id);
                        self.store.yt.insert(0, e);
                        self.store_dirty = true;
                    }
                    self.set_note("SAVED TO MY CLIPS");
                }
            }
            Action::ScKeep(id) => {
                if let Some(e) = self.sc_results.iter().find(|e| e.id == id).cloned() {
                    if !self.store.sc.iter().any(|x| x.id == id) {
                        self.here.insert(id);
                        self.store.sc.insert(0, e);
                        self.store_dirty = true;
                    }
                    self.set_note("SAVED TO MY SOUNDCLOUD");
                }
            }
            Action::GoOffline(on) => {
                self.go_offline = on;
                crate::api::OFFLINE.store(on, std::sync::atomic::Ordering::Relaxed);
                self.dirty = true;
                if !on {
                    // the covers that couldn't load offline come in now
                    self.images.retry_failed();
                }
                if on {
                    // as if there were no internet: your library as kept here, only what is on this computer
                    let _ = self.tx.send(crate::Msg::NoInternet);
                    self.set_note("OFFLINE MODE: ONLY MUSIC ON THIS COMPUTER. GO ONLINE TO USE TIDAL");
                } else {
                    self.set_note("GOING ONLINE...");
                    let (api, tx, ctx) = (self.api.clone(), self.tx.clone(), self.ctx.clone());
                    std::thread::spawn(move || {
                        let _ = match api.refresh_info() {
                            Ok(()) => tx.send(crate::Msg::LoggedIn),
                            Err(e) if crate::api::no_internet(&e) => tx.send(crate::Msg::NoInternet),
                            Err(e) => tx.send(crate::Msg::NeedLogin(e)),
                        };
                        ctx.request_repaint();
                    });
                }
            }
            Action::Offline(on) => {
                self.offline = on;
                self.dirty = true;
                if on {
                    self.sec = Sec::Files;
                } else {
                    self.sec = Sec::Tidal;
                }
                self.serial += 1;
            }
            Action::GetYtDlp => {
                if self.yt_busy {
                    return;
                }
                self.yt_busy = true;
                self.yt_msg = "DOWNLOADING YT-DLP...".to_string();
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let _ = tx.send(Msg::YtTool(sources::ytdlp_download()));
                    ctx.request_repaint();
                });
            }
            Action::RemoveExt(id) => {
                let file = self.store.files.iter().any(|e| e.id == id);
                if file && !self.store.removed_files.contains(&id) {
                    self.store.removed_files.push(id);
                }
                if self.cur_track().map(|c| c.id) == Some(id) {
                    self.player.send(Cmd::Stop);
                }
                self.store.yt.retain(|e| e.id != id);
                self.store.sc.retain(|e| e.id != id);
                self.liked.remove(&id);
                // (the file list, hearts, playlists, tunes, recent and the queue)
                self.file_gone(id);
                cache::remove(id);
                self.set_note(if file { "REMOVED FROM BACKLINE (THE FILE ITSELF IS STILL ON YOUR DISK)" } else { "REMOVED, AND ITS SAVED COPY DELETED" });
                self.store_dirty = true;
                self.serial += 1;
            }

            // ---- tunes
            Action::PickTune(t) => {
                self.set_note(&format!("PICK A TUNE FOR: {}", t.title.to_uppercase()));
                self.pick = Some(t);
                self.sec = Sec::Tunes;
                self.tune_open = None;
                self.serial += 1;
            }
            Action::AddToTune(i, t) => {
                let v = self.version_of(&t);
                self.add_version(i, v);
                self.pick = None;
            }
            Action::NewTuneFrom(t) => {
                let name = tune_name_from(&t.title);
                let i = self.new_tune_named(&name);
                let v = self.version_of(&t);
                self.add_version(i, v);
                self.pick = None;
                self.tune_open = Some(i);
                self.chart_pick = Some(i);
                self.chart_live = false;
                self.sec = Sec::Tunes;
                self.serial += 1;
            }
            Action::TuneDetails(i) => {
                let Some(name) = self.store.tunes.get(i).map(|t| t.name.trim().to_string()) else { return };
                let artist = self.store.tunes[i].versions.first().map(|v| v.artist.clone());
                if name.is_empty() || !self.details_tried.insert(name.to_lowercase()) {
                    return;
                }
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let w = sources::lookup_work(&name, artist.as_deref());
                    let _ = tx.send(Msg::TuneDetails(name, w));
                    ctx.request_repaint();
                });
            }
            Action::NewTune => {
                let name = self.new_tune.trim().to_string();
                if name.is_empty() {
                    return;
                }
                let i = self.new_tune_named(&name);
                self.new_tune.clear();
                if let Some(t) = self.pick.take() {
                    let v = self.version_of(&t);
                    self.add_version(i, v);
                }
                self.tune_open = Some(i);
                // the lead sheet shows the new tune (blank until its changes are found or typed)
                self.chart_pick = Some(i);
                self.chart_live = false;
                self.serial += 1;
            }
            Action::OpenTune(i) => {
                self.tune_open = i;
                // a tune opened: the lead sheet is its again, the whole of it
                self.chart_sel = None;
                if i.is_some() {
                    self.prac_chart = None;
                }
                // the tune's own tempo for the metronome and the band (kept with it from now on)
                if let Some(b) = i.and_then(|i| self.tune_bpm(i)) {
                    self.mt_bpm = b;
                    self.mt_add = 0.0;
                    if self.cur.is_none() || self.stopped {
                        self.bpm = b;
                    }
                    if let Some(t) = i.and_then(|i| self.store.tunes.get_mut(i)) {
                        if t.bpm == 0 {
                            t.bpm = b;
                            self.store_dirty = true;
                        }
                    }
                }
                if i.is_some() {
                    self.chart_pick = i;
                    self.chart_live = false;
                }
                self.edit_ver = None;
                self.del_arm = false;
                self.chart_edit = false;
                self.ed.id = 0;
                self.serial += 1;
            }
            Action::RemoveTune(i) => {
                if !self.del_arm {
                    self.del_arm = true;
                    return;
                }
                self.del_arm = false;
                if i < self.store.tunes.len() {
                    self.store.tunes.remove(i);
                    self.store_dirty = true;
                }
                self.tune_open = None;
                self.edit_ver = None;
                self.serial += 1;
            }
            Action::RemoveVersion(i, vi) => {
                if let Some(t) = self.store.tunes.get_mut(i) {
                    if vi < t.versions.len() {
                        t.versions.remove(vi);
                        self.store_dirty = true;
                    }
                }
                self.edit_ver = None;
            }
            Action::PlayTune(i, shuffled) => {
                let vs: Vec<Version> = match self.store.tunes.get(i) {
                    Some(t) => t.versions.clone(),
                    None => return,
                };
                if vs.is_empty() {
                    self.set_note("NO RECORDINGS YET. ADD ONE FIRST");
                    return;
                }
                for v in &vs {
                    self.register(v);
                }
                let tracks: Vec<Track> = vs.iter().map(|v| v.to_track()).collect();
                if shuffled {
                    self.apply(Action::PlayShuffled(tracks));
                } else {
                    self.apply(Action::Play(tracks, 0));
                }
            }
            Action::AddCurrentToTune(i) => match self.cur_track() {
                Some(t) => {
                    let v = self.version_of(&t);
                    self.add_version(i, v);
                }
                None => self.set_note("PLAY A RECORDING FIRST"),
            },
            Action::FindChart(i, alt) => {
                let Some(t) = self.store.tunes.get(i) else { return };
                let name = t.name.clone();
                if self.chart_busy {
                    return;
                }
                self.chart_busy = true;
                self.chart_sugg.clear();
                let query = alt.unwrap_or_else(|| name.clone());
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let _ = tx.send(Msg::Chart(name, sources::find_chart(&query)));
                    ctx.request_repaint();
                });
            }
            Action::LiveChart(title, alt) => {
                if self.chart_busy {
                    return;
                }
                self.chart_busy = true;
                self.live_q = title.clone();
                self.live = None;
                self.chart_sugg.clear();
                let query = alt.unwrap_or_else(|| title.clone());
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let _ = tx.send(Msg::Live(title, sources::find_chart(&query)));
                    ctx.request_repaint();
                });
            }
            Action::SaveLive => {
                let (Some(t), Some((chart, key, composer))) = (self.cur_track(), self.live.clone()) else { return };
                let name = tune_name_from(&t.title);
                let i = self.new_tune_named(&name);
                let v = self.version_of(&t);
                self.add_version(i, v);
                let tu = &mut self.store.tunes[i];
                tu.chart = chart;
                tu.key = key;
                if tu.written_by.is_empty() && !composer.is_empty() {
                    tu.written_by = composer;
                }
                self.chart_live = false;
                self.chart_pick = Some(i);
                self.store_dirty = true;
                self.set_note("SAVED TO YOUR TUNES");
            }
            Action::ToggleNumerals => self.chart_rn = !self.chart_rn,
            Action::Status(i, n) => {
                if let Some(t) = self.store.tunes.get_mut(i) {
                    t.status = n.min(3);
                    self.store_dirty = true;
                }
            }
            Action::Knob(k, d) => {
                let v = self.knob_val(k) + d as f32 * knob_step(k);
                self.knob_set(k, v);
            }
            Action::SetKnob(k, text) if matches!(k, Knob::Beats) && text.contains('/') => {
                // a whole meter typed in one go, like 11/17
                let mut it = text.split('/').map(|s| s.trim().parse::<f32>().ok());
                match (it.next().flatten(), it.next().flatten()) {
                    (Some(b), Some(u)) => {
                        self.knob_set(Knob::Beats, b);
                        self.knob_set(Knob::Unit, u);
                    }
                    _ => self.set_note("TYPE A METER LIKE 11/17"),
                }
            }
            Action::SetKnob(k, text) => {
                let v = if matches!(k, Knob::LoopA | Knob::LoopB) {
                    parse_time(&text)
                } else {
                    text.trim().trim_end_matches('%').trim().parse::<f32>().ok()
                };
                match v {
                    _ if text.trim().is_empty() => {}
                    Some(v) => self.knob_set(k, v),
                    None => self.set_note("THAT IS NOT A NUMBER"),
                }
            }
            Action::Opt(o) => self.opt(o),
            Action::LookUp(i) => {
                if self.look_busy {
                    return;
                }
                let Some(t) = self.store.tunes.get(i) else { return };
                let name = t.name.clone();
                let known: Vec<String> = self
                    .store
                    .tunes
                    .iter()
                    .flat_map(|t| t.versions.iter().filter(|v| v.stars >= 3).map(|v| v.artist.clone()))
                    .collect();
                self.look_busy = true;
                self.look_for = name.clone();
                self.look_tracks.clear();
                let (api, tx, ctx) = (self.api.clone(), self.tx.clone(), self.ctx.clone());
                let want_style = self.look_style;
                // something typed beside LOOK UP is searched instead of the tune's own name
                let typed = self.look_q.trim().to_string();
                let tune_name = name.clone();
                // someone who recorded it, so a title other songs share finds the right one
                let artist = if typed.is_empty() {
                    self.store.tunes.get(i).and_then(|t| t.versions.first()).map(|v| v.artist.clone())
                } else {
                    None
                };
                let name = if typed.is_empty() { name } else { typed };
                std::thread::spawn(move || {
                    let info = sources::lookup_work(&name, artist.as_deref());
                    // a tune in the jazz standards list is jazz (and that list knows who wrote it)
                    let listed = sources::find_chart(&name).ok().map(|c| c.2);
                    let style = match want_style {
                        0 => u8::from(listed.is_some()),
                        1 => 1,
                        2 => 2,
                        _ => 0,
                    };
                    let composer = listed.unwrap_or_else(|| info.as_ref().map(|w| w.writers.clone()).unwrap_or_default());
                    let composer = composer.split([',', '\n']).next().unwrap_or("").replace(" (words)", "");
                    let mut tracks = api.popular(&name, &known, style, composer.trim()).unwrap_or_default();
                    // no Tidal (or nothing there): recordings from YouTube instead, so everyone gets suggestions
                    let mut found = Vec::new();
                    if tracks.is_empty() && sources::ytdlp_path().is_some() {
                        let q = match style {
                            1 if !composer.trim().is_empty() => format!("{} {} jazz", name, composer.trim()),
                            1 => format!("{} jazz standard", name),
                            2 => format!("{} gospel", name),
                            _ => name.clone(),
                        };
                        if let Ok(mut list) = sources::yt_search(&q) {
                            // recordings, not lessons or play-alongs
                            list.retain(|e| {
                                let t = e.title.to_lowercase();
                                e.dur < 720.0
                                    && !["lesson", "tutorial", "play-along", "play along", "playalong", "how to", "backing track", "tab)", "tabs"]
                                        .iter()
                                        .any(|w| t.contains(w))
                            });
                            tracks = list.iter().map(|e| (e.to_track(), 0)).collect();
                            found = list;
                        }
                    }
                    let _ = tx.send(Msg::Lookup(tune_name, info, tracks, found));
                    ctx.request_repaint();
                });
            }

            // ---- sections and loops
            Action::SaveSection => {
                let (Some(a), Some(b)) = (self.loop_a, self.loop_b) else {
                    self.set_note("SET LOOP A AND B FIRST");
                    return;
                };
                let Some(t) = self.cur_track() else { return };
                if self.is_demo(t.id) {
                    self.set_note("NOT DURING THE TOUR");
                    return;
                }
                let typed = self.sec_name.trim().to_string();
                let list = self.store.sections.entry(t.id).or_default();
                // the same loop again: a new name renames it, no name just says so
                if let Some(s) = list.iter_mut().find(|s| (s.a - a).abs() < 0.05 && (s.b - b).abs() < 0.05) {
                    if typed.is_empty() || typed.eq_ignore_ascii_case(&s.name) {
                        let n = s.name.to_uppercase();
                        self.set_note(&format!("THIS LOOP IS ALREADY SAVED AS {}", n));
                    } else {
                        s.name = typed.clone();
                        self.sec_name.clear();
                        self.store_dirty = true;
                        self.set_note(&format!("RENAMED: {}", typed.to_uppercase()));
                    }
                    return;
                }
                let name = if typed.is_empty() { format!("LOOP {}", list.len() + 1) } else { typed };
                list.push(Section { name: name.clone(), a, b, saved: store::today_str(), ..Default::default() });
                self.keep_for_practice(t.id);
                self.remember_song(&t);
                // saving a loop is practice: the song is in this week's list
                let date = store::today_str();
                self.store.day_mut(&date).songs.entry(t.id).or_insert(0);
                self.sec_name.clear();
                self.store_dirty = true;
                self.set_note(&format!("SAVED: {}", name.to_uppercase()));
            }
            Action::GoSection(i) => {
                let Some(t) = self.cur_track() else { return };
                let Some(s) = self.store.sections.get(&t.id).and_then(|l| l.get(i)).cloned() else { return };
                self.practice = true;
                self.loop_a = Some(s.a);
                self.loop_b = Some(s.b);
                self.loop_on = true;
                self.loop_ident = None;
                self.loop_saved = Some((t.id, i));
                self.sync_loop();
                self.player.send(Cmd::Seek(s.a));
            }
            Action::PlaySaved(v, lp) => self.play_saved(&v, lp),
            Action::PlayAt(v, t) => {
                let here = self.cur_track().map(|t| t.id) == Some(v.id);
                self.play_saved(&v, None);
                if here || !self.buffering {
                    self.player.send(Cmd::Seek(t));
                } else {
                    self.pending_seek = Some(t);
                }
            }
            Action::EditLoop(id, i, name, note, group) => {
                if let Some(s) = self.store.sections.get_mut(&id).and_then(|l| l.get_mut(i)) {
                    if !name.trim().is_empty() {
                        s.name = name.trim().to_string();
                    }
                    s.note = note.trim().to_string();
                    s.group = group.trim().to_string();
                    self.store_dirty = true;
                }
                let g = group.trim().to_string();
                if !g.is_empty() && !self.store.groups().contains(&g) {
                    let mut l = self.store.groups();
                    l.push(g);
                    self.store.loop_groups = Some(l);
                }
            }
            Action::LoopGroup(id, i, group) => {
                if let Some(s) = self.store.sections.get_mut(&id).and_then(|l| l.get_mut(i)) {
                    s.group = group.trim().to_string();
                    self.store_dirty = true;
                }
            }
            Action::ForgetLoop(id, i) => {
                // (the saved loops are counted again: which one is current, found afresh)
                self.loop_saved = None;
                if let Some(l) = self.store.sections.get_mut(&id) {
                    if i < l.len() {
                        l.remove(i);
                        self.store_dirty = true;
                    }
                    if l.is_empty() {
                        self.store.sections.remove(&id);
                    }
                }
            }
            Action::DeleteSection(i) => {
                self.loop_saved = None;
                if let Some(t) = self.cur_track() {
                    if let Some(l) = self.store.sections.get_mut(&t.id) {
                        if i < l.len() {
                            l.remove(i);
                            self.store_dirty = true;
                        }
                        if l.is_empty() {
                            self.store.sections.remove(&t.id);
                        }
                    }
                }
            }

            // ---- practice tools
            Action::ToggleLibrary => {
                self.lib_hidden = !self.lib_hidden;
                // (the chart's page is in the library: while it's hidden, the chart takes the queue's window)
                let has_chart = self.chart_tune().map(|i| !self.store.tunes[i].chart.is_empty()).unwrap_or(false);
                self.rtab = if self.lib_hidden && has_chart { 1 } else { 0 };
                // with a skin, the window narrows to the player's column (the deck, playlist and practice panel
                // standing alone, as Winamp's do), and is as wide as it was when the library comes back
                if self.wa_art.is_some() && !self.mini && !self.fullscreen {
                    let (inner, max) = self.ctx.input(|i| (i.viewport().inner_rect, i.viewport().maximized.unwrap_or(false)));
                    if let Some(r) = inner {
                        if self.lib_hidden {
                            self.lib_shown_w = Some((r.width(), max));
                            if max {
                                self.ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
                            }
                            let w = (275.0 * self.stack_k).round();
                            self.ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(w, r.height())));
                        } else if let Some((w, was_max)) = self.lib_shown_w.take() {
                            self.ctx.send_viewport_cmd(if was_max {
                                egui::ViewportCommand::Maximized(true)
                            } else {
                                egui::ViewportCommand::InnerSize(Vec2::new(w, r.height()))
                            });
                        }
                    }
                }
                self.set_note(if self.lib_hidden { "LIBRARY HIDDEN" } else { "LIBRARY" });
            }
            Action::Continue => {
                let Some(l) = self.store.last.clone() else { return };
                // the queue it was in when there is one, else just the song
                let (queue, index) = if l.queue.is_empty() { (vec![l.version.clone()], 0) } else { (l.queue.clone(), l.index) };
                for v in &queue {
                    self.register(v);
                }
                self.queue = Arc::new(queue.iter().map(|v| v.to_track()).collect());
                self.orig_queue = None;
                self.shuffle = false;
                self.practice = true;
                self.play_index(index.min(self.queue.len().saturating_sub(1)));
                // where you left off, at the speed you left it
                self.speed = if l.speed == 0 { 100 } else { l.speed.clamp(25, 250) };
                self.sync_loop();
                if l.pos > 3.0 {
                    let t = (l.pos - 2.0).max(0.0);
                    if self.buffering {
                        self.pending_seek = Some(t);
                    } else {
                        self.player.send(Cmd::Seek(t));
                    }
                }
            }
            Action::Trainer => {
                self.trainer = !self.trainer;
                self.passes = 0;
                if self.trainer {
                    self.practice = true;
                    self.set_note(&format!("EVERY {} PASSES THE SPEED GOES UP {}%", self.trainer_n, self.trainer_step));
                } else {
                    self.set_note("SPEED TRAINER OFF");
                }
                self.sync_loop();
            }
            Action::TapTempo => {
                let now = Instant::now();
                if let Some(l) = self.taps.last() {
                    if now.duration_since(*l) > Duration::from_millis(2000) {
                        self.taps.clear();
                    }
                }
                self.taps.push(now);
                if self.taps.len() > 8 {
                    self.taps.remove(0);
                }
                if self.taps.len() >= 3 {
                    let span = now.duration_since(self.taps[0]).as_secs_f32();
                    let n = (self.taps.len() - 1) as f32;
                    let heard = 60.0 / (span / n).max(0.05);
                    let factor = if self.practice { self.speed as f32 / 100.0 } else { 1.0 };
                    // tapping finds the music's tempo: the song's and the metronome's both take it
                    self.knob_set(Knob::Bpm, (heard / factor).round());
                    self.knob_set(Knob::MetroBpm, (heard / factor).round());
                } else {
                    self.set_note("KEEP TAPPING...");
                }
            }
            Action::CountIn => {
                self.count_in = match self.count_in {
                    0 => 2,
                    2 => 4,
                    _ => 0,
                };
                if self.count_in > 0 && self.bpm == 0 {
                    self.set_note("THE COUNT-OFF NEEDS A TEMPO. TAP IT OR SET THE BPM");
                }
                self.sync_loop();
            }
            Action::Chan => {
                self.chan = (self.chan + 1) % CHAN_NAMES.len() as u32;
                self.practice = true;
                self.sync_loop();
                self.set_note(&format!("EAR: {}", CHAN_NAMES[self.chan as usize]));
            }
            Action::Transpose(d) => {
                self.semis = (self.semis + d).clamp(-1200, 1200);
                if self.cur.is_some() {
                    self.practice = true;
                }
                self.sync_loop();
                self.set_note(&format!("PITCH {:+.2} SEMITONES (SPEED UNCHANGED)", self.semis as f32 / 100.0));
            }
            Action::Export => self.export_wav(),
            Action::ExportPanel => {
                self.export_open = !self.export_open;
                if self.export_open {
                    (self.timer_open, self.metro_open) = (false, false);
                }
            }
            Action::ExportFolder => {
                let (tx, ctx, at) = (self.tx.clone(), self.ctx.clone(), self.export_folder());
                std::thread::spawn(move || {
                    if let Some(d) = rfd::FileDialog::new().set_title("Save exports in this folder").set_directory(&at).pick_folder() {
                        let _ = tx.send(Msg::ExportDir(d));
                        ctx.request_repaint();
                    }
                });
            }

            // ---- diary
            Action::SaveEntry => {
                let today = if self.diary_sel == 0 { store::today_str() } else { store::date_str(self.diary_sel) };
                let default_mins = self.store.secs_on(&today) / 60;
                let mins = self.f_mins.trim().parse::<u32>().unwrap_or(default_mins);
                let bpm = self.f_bpm.trim().parse::<u32>().unwrap_or(0);
                let tune = if self.f_tune.trim().is_empty() { self.practice_label() } else { self.f_tune.trim().to_string() };
                let note = self.f_note.trim().to_string();
                match self.entry_edit.take().filter(|i| *i < self.store.entries.len()) {
                    // an entry changed: it keeps its day
                    Some(i) => {
                        let e = &mut self.store.entries[i];
                        (e.tune, e.mins, e.bpm, e.note) = (tune, mins, bpm, note);
                    }
                    None => {
                        // a logged day with no tracked time still counts on the calendar
                        if mins > 0 && self.store.secs_on(&today) == 0 {
                            *self.store.day_mut(&today).secs.entry(tune.clone()).or_insert(0) += mins * 60;
                        }
                        self.store.entries.push(Entry { date: today, tune, mins, bpm, note });
                    }
                }
                self.f_mins.clear();
                self.f_bpm.clear();
                self.f_note.clear();
                self.f_tune.clear();
                self.show_form = false;
                self.ed.id = 0;
                self.store_dirty = true;
                self.set_note("ADDED TO THE DIARY");
            }
            Action::RightTab(n) => self.rtab = n,
            Action::OpenChart => self.open_chart(),
            Action::StemGet => {
                if self.stem_busy != 0 {
                    return;
                }
                self.stem_busy = 1;
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let _ = tx.send(Msg::StemTool(stems::download_tool()));
                    ctx.request_repaint();
                });
            }
            Action::StemSplit => {
                if self.stem_busy != 0 {
                    return;
                }
                let Some(t) = self.cur_track() else {
                    self.set_note("PLAY A TRACK FIRST");
                    return;
                };
                self.stem_busy = 2;
                self.set_note("SPLITTING. KEEP PRACTICING, IT TAKES A FEW MINUTES");
                let id = t.id;
                // the stored copy, or (not stored: storing tracks is off, or it never finished) the track fetched
                // again for the split
                let src = self.srcmap.get(&t.id).cloned();
                let (lossless, api) = (self.prefer_lossless, self.api.clone());
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    api::log(&format!("stems: split {} asked for", id));
                    let r = crate::fetch_audio(&api, id, lossless, false, src).and_then(|b| {
                        api::log(&format!("stems: {} bytes of audio, splitting", b.len()));
                        stems::separate(b, id)
                    });
                    api::log(&format!("stems: {} -> {:?}", id, r));
                    let _ = tx.send(Msg::Stems(id, r));
                    ctx.request_repaint();
                });
            }
            Action::StemCancel => stems::CANCEL.store(true, std::sync::atomic::Ordering::Relaxed),
            Action::StemToggle(i) => {
                if i < 4 {
                    let mut on = self.stem_on;
                    on[i] = !on[i];
                    self.apply_mix(on);
                }
            }
            Action::StemSolo(i) => {
                if i < 4 {
                    let mut on = [false; 4];
                    on[i] = true;
                    self.apply_mix(on);
                }
            }
            Action::StemAll => self.apply_mix([true; 4]),
            Action::StemClear => {
                let _ = std::fs::remove_dir_all(api::config_dir().join("stems"));
                stems::forget_split();
                self.stem_on = [true; 4];
                self.set_note("ALL STEMS DELETED");
            }
            Action::StemDelete(id) => {
                // back to the whole song first if a mix of stems is playing
                if self.cur_track().is_some_and(|t| t.id == id) && self.stem_on != [true; 4] {
                    self.apply_mix([true; 4]);
                }
                let _ = std::fs::remove_dir_all(stems::stems_dir(id));
                stems::forget_split();
                self.stem_on = [true; 4];
                self.set_note("THIS SONG'S STEMS DELETED");
            }
            Action::StemOpen(id) => stems::open_folder(id),
            Action::CmImport(own) => {
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let picked = if own {
                        rfd::FileDialog::new()
                            .set_title("Import transcriptions")
                            .add_filter("Backline transcription", &["json"])
                            .pick_files()
                    } else {
                        rfd::FileDialog::new().set_title("Import Guitar Pro files").add_filter("Guitar Pro", &["gp"]).pick_files()
                    };
                    let Some(files) = picked else { return };
                    let mut done = Vec::new();
                    let mut errs = Vec::new();
                    for f in files {
                        let stem = f.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                        let r = if own {
                            crate::transcription::load(&f).and_then(|t| {
                                // its recording too, when it came with one beside it
                                let audio =
                                    (!t.audio.is_empty()).then(|| f.with_file_name(&t.audio)).and_then(|a| std::fs::read(a).ok());
                                crate::transcription::save_in(None, t, audio.as_deref())
                            })
                        } else {
                            std::fs::read(&f)
                                .map_err(|e| e.to_string())
                                .and_then(|b| crate::transcription::from_gp(&b, &stem))
                                .and_then(|(t, a)| crate::transcription::save_in(None, t, a.as_deref()))
                        };
                        match r {
                            Ok(p) => done.push(p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()),
                            Err(e) => errs.push(format!("{}: {}", stem, e)),
                        }
                    }
                    let _ = tx.send(crate::Msg::CmImported(if done.is_empty() { Err(errs.join("; ")) } else { Ok(done) }));
                    ctx.request_repaint();
                });
            }
            Action::PomoHold => {
                if matches!(self.pomo, 1 | 2) {
                    self.pomo_left = self.pomo_end.map_or(0, |e| e.saturating_duration_since(Instant::now()).as_secs());
                    self.pomo_was = self.pomo;
                    self.pomo = 4;
                    self.pomo_end = None;
                    self.set_note("FOCUS TIMER PAUSED. RESUME WHEN READY");
                } else if self.pomo == 4 {
                    self.pomo = self.pomo_was.clamp(1, 2);
                    self.pomo_end = Some(Instant::now() + Duration::from_secs(self.pomo_left.max(1)));
                    self.set_note("FOCUS TIMER RESUMED");
                }
                self.dirty = true;
            }
            Action::Pomo => {
                self.dirty = true;
                if matches!(self.pomo, 1 | 2 | 4) {
                    self.pomo = 0;
                    self.pomo_end = None;
                    self.set_note("FOCUS TIMER STOPPED");
                } else {
                    self.pomo_n = if self.pomo == 3 { self.pomo_n + 1 } else { 1 };
                    self.pomo = 1;
                    self.pomo_end = Some(Instant::now() + Duration::from_secs(self.pomo_focus as u64 * 60));
                    self.set_note(&format!("FOCUS {} OF {}. ONE THING AT A TIME", self.pomo_n, self.pomo_cycles));
                }
                self.pomo_flash = None;
            }
            Action::TimerPanel => self.timer_open = !self.timer_open,
            Action::PlayAlong(name) => {
                let q: String = format!("{} backing track play along", name)
                    .bytes()
                    .map(|b| if b.is_ascii_alphanumeric() { (b as char).to_string() } else { format!("%{:02X}", b) })
                    .collect();
                let _ = webbrowser::open(&format!("https://www.youtube.com/results?search_query={}", q));
                self.set_note("OPENED YOUTUBE IN YOUR BROWSER");
            }
            Action::MetroPanel => self.metro_open = !self.metro_open,
            Action::ImportIreal(target) => {
                let text = self.ireal_in.trim().to_string();
                match crate::chart::parse_ireal_url(&text) {
                    Err(e) => self.set_err(format!("CAN'T READ THAT LINK: {}", e)),
                    Ok(songs) if songs.is_empty() => self.set_note("NO SONGS IN THAT LINK"),
                    Ok(songs) => {
                        let mut count = 0;
                        let single = songs.len() == 1;
                        let mut last = None;
                        for s in songs {
                            let idx = match target {
                                Some(t) if single && t < self.store.tunes.len() => t,
                                _ => self.new_tune_named(&s.title),
                            };
                            let tune = &mut self.store.tunes[idx];
                            tune.chart = crate::chart::to_friendly(&s.measures, &s.time);
                            if tune.key.is_empty() {
                                tune.key = s.key.clone();
                            }
                            if tune.bpm == 0 {
                                tune.bpm = s.bpm;
                            }
                            if tune.written_by.is_empty() && !s.composer.is_empty() {
                                tune.written_by = s.composer.clone();
                            }
                            if tune.style.is_empty() && !s.style.is_empty() {
                                tune.style = s.style.clone();
                            }
                            last = Some(idx);
                            count += 1;
                        }
                        self.store_dirty = true;
                        self.ireal_in.clear();
                        self.chart_edit = false;
                        self.ed.id = 0;
                        if let Some(i) = last {
                            self.tune_open = Some(i);
                        }
                        self.set_note(&format!("IMPORTED {} CHART{}", count, if count == 1 { "" } else { "S" }));
                    }
                }
            }
            _ => {}
        }
    }
}
