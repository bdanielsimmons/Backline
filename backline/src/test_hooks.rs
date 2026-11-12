//! Test copies: a copy started with BACKLINE_OPEN=<page> opens on that page, set up to show it (for pictures of a
//! layout and for recording demos), and the pictures it takes (BACKLINE_SHOT). A copy started the usual way runs
//! none of this.

use super::*;

impl App {
    /// BACKLINE_OPEN=<page> (and the BACKLINE_* that go with some pages): open on that page. Called once, at start.
    pub(crate) fn open_for_test(&mut self) {
        let app = self;
        // BACKLINE_OPEN=practice / lists: start with that open (for checking layouts and recording demos)
        // (a test copy opened on a page shows that page, whatever side page it was left on)
        if std::env::var_os("BACKLINE_OPEN").is_some() {
            app.side = None;
        }
        // (BACKLINE_LOOK=<n>: wearing Backline's look n; "custom": your Custom one, its editor open)
        match std::env::var("BACKLINE_LOOK").as_deref() {
            Ok("custom") => app.edit_look(),
            Ok(n) => {
                if let Ok(n) = n.parse::<usize>() {
                    app.wear_look(n);
                }
            }
            _ => {}
        }
        // (BACKLINE_HIDELIB=1: the library hidden; give it a narrow BACKLINE_SIZE, as the window narrows to the player's
        // column)
        if std::env::var_os("BACKLINE_HIDELIB").is_some() {
            app.lib_hidden = true;
        }
        // (BACKLINE_PRACTICE=1: in practice mode, whatever page it opens on)
        if std::env::var_os("BACKLINE_PRACTICE").is_some() {
            app.practice = true;
        }
        match std::env::var("BACKLINE_OPEN").as_deref() {
            Ok("practice") => app.practice = true,
            Ok("prefs") => app.show_prefs = true,
            // the practice panel, the playing song's second loop open to edit (its note box)
            Ok("loopnote") => {
                app.practice = true;
                if let Some(id) = app.queue.first().map(|t| t.id) {
                    if let Some(s) = app.store.sections.get(&id).and_then(|l| l.get(1)).cloned() {
                        app.clip_edit = Some((id, 1));
                        app.edit_in_practice = true;
                        app.clip_name = s.name;
                        app.clip_note = s.note;
                        app.clip_group = s.group;
                    }
                }
            }
            // pick up the last song where it was left (as CONTINUE does)
            Ok("continue") => app.apply(Action::Continue),
            Ok("zoom") => {
                app.practice = true;
                if let Some(p) = tour_song::ready() {
                    let t = sources::ext_for_file(&p).to_track();
                    app.srcmap.insert(t.id, Src::File(p));
                    app.queue = Arc::new(vec![t]);
                    app.play_index(0);
                }
                app.wv_zoom = 6.0;
            }
            Ok("record") => {
                app.practice = true;
                app.more = true;
                app.mtab = 4;
            }
            // the practice panel's other tabs
            Ok(t @ ("trainer" | "pitch" | "stems")) => {
                app.practice = true;
                app.more = true;
                app.mtab = ["trainer", "pitch", "stems"].iter().position(|x| *x == t).unwrap_or(0) as u8;
                // with the tour song playing, so the tab shows what it does (BACKLINE_REC_A=452: tuned to that A;
                // BACKLINE_SONG=<file>: that song instead; BACKLINE_MENU=export: with the EXPORT panel open, on the
                // loop BACKLINE_LOOP=<a>-<b> seconds)
                if std::env::var("BACKLINE_MENU").as_deref() == Ok("export") {
                    app.export_open = true;
                }
                let song = std::env::var_os("BACKLINE_SONG").map(std::path::PathBuf::from).or_else(tour_song::ready);
                if let Some(p) = song {
                    let t = sources::ext_for_file(&p).to_track();
                    if let Some(a) = std::env::var("BACKLINE_REC_A").ok().and_then(|a| meta::parse_hz(&a)) {
                        meta::set_a4(t.id, Some(a));
                    }
                    // (BACKLINE_CHAN=<n>: in that ear mode)
                    if let Some(c) = std::env::var("BACKLINE_CHAN").ok().and_then(|c| c.parse::<u32>().ok()) {
                        app.chan = c % extras::CHAN_NAMES.len() as u32;
                    }
                    app.srcmap.insert(t.id, Src::File(p));
                    app.queue = Arc::new(vec![t]);
                    app.play_index(0);
                    if let Some((a, b)) = std::env::var("BACKLINE_LOOP").ok().and_then(|l| {
                        let (a, b) = l.split_once('-')?;
                        Some((a.parse::<f32>().ok()?, b.parse::<f32>().ok()?))
                    }) {
                        (app.loop_a, app.loop_b) = (Some(a), Some(b));
                    }
                }
            }
            // (BACKLINE_PL=<n>: with your playlist n open)
            Ok("lists") => {
                app.sec = Sec::Lists;
                app.pl_open = std::env::var("BACKLINE_PL").ok().and_then(|n| n.parse().ok());
            }
            // the library's ARTISTS tab (BACKLINE_OPEN=artist opens an artist's page once the library is in: Msg::Page)
            Ok("artists") => {
                app.sec = Sec::Tidal;
                app.lib_tab = Tab::Artists;
            }
            Ok("recent") => {
                app.sec = Sec::Tidal;
                app.lib_tab = Tab::Recent;
            }
            Ok("loops") => app.sec = Sec::Loops,
            // LOOPS > ALL: every song, under the months' headings
            Ok("loopsall") => {
                app.sec = Sec::Loops;
                app.loops_all = true;
            }
            // the LOOPS page open on the song with the most loops
            Ok("loopsong") => {
                app.sec = Sec::Loops;
                app.loops_pick = app.store.sections.iter().max_by_key(|(id, l)| (l.len(), **id)).map(|(id, _)| *id);
            }
            Ok("groups") => {
                app.sec = Sec::Loops;
                app.loops_mode = 2;
            }
            // the LOOPS page on the song with the most bookmarks, the second one's name and note being edited
            Ok("loopmarks") => {
                app.sec = Sec::Loops;
                app.loops_all = true;
                app.loops_pick = app.store.marks.iter().max_by_key(|(id, l)| (l.len(), **id)).map(|(id, _)| *id);
                if let Some(m) = app.loops_pick.and_then(|id| Some((id, app.store.marks[&id].get(1)?.clone()))) {
                    app.mark_edit = Some((m.0, 1));
                    app.clip_name = m.1.name;
                    app.clip_note = format!("{}
Second line: comp lighter here.", m.1.note);
                }
            }
            Ok("moves") => {
                app.sec = Sec::Chords;
                app.chords_tab = 7;
                // BACKLINE_MV_MOVE=<n>: that move
                if let Some(m) = std::env::var("BACKLINE_MV_MOVE").ok().and_then(|s| s.parse::<usize>().ok()) {
                    app.mv_move = m;
                }
            }
            Ok("prog") => {
                app.sec = Sec::Chords;
                app.chords_tab = 8;
                // BACKLINE_PROG_FAM=<n>: that family (5 = GOSPEL)
                if let Some(f) = std::env::var("BACKLINE_PROG_FAM").ok().and_then(|s| s.parse::<usize>().ok()) {
                    app.prog_fam = f;
                }
            }
            Ok("news") => app.apply(Action::News),
            Ok("mini") => app.apply(Action::ToggleMini),
            Ok("mini_eq_pl") => {
                app.apply(Action::ToggleMini);
                app.show_eq = true;
                app.queue_hidden = false;
            }
            Ok("mine") => app.sec = Sec::Mine,
            Ok("yt") => app.sec = Sec::Yt,
            Ok("diary") => app.sec = Sec::Diary,
            Ok("diarywrite") => {
                app.sec = Sec::Diary;
                app.ed.id = crate::tools::F_DIARY_NOTE;
            }
            Ok("diarynotes") => {
                app.sec = Sec::Diary;
                app.diary_all = 30;
                // (BACKLINE_NOTES_Q=<words>: searched; BACKLINE_NOTES_MODE=0|1|2: a year, month or day at a time)
                app.notes_q = std::env::var("BACKLINE_NOTES_Q").unwrap_or_default();
                if let Some(m) = std::env::var("BACKLINE_NOTES_MODE").ok().and_then(|v| v.parse::<u8>().ok()) {
                    app.notes_mode = m.min(2);
                }
            }
            Ok("routine") => {
                app.sec = Sec::Diary;
                app.diary_tab = 1;
            }
            // Now Playing with the tour song (BACKLINE_VIS=<preset>: with visuals in place of the cover)
            Ok("nowplaying") => {
                if let Some(p) = tour_song::ready() {
                    let t = sources::ext_for_file(&p).to_track();
                    app.srcmap.insert(t.id, Src::File(p));
                    app.queue = Arc::new(vec![t]);
                    app.play_index(0);
                }
                app.art_view = true;
                if let Ok(v) = std::env::var("BACKLINE_VIS") {
                    app.vis_preset = visuals::find(&v);
                    app.vis_np = true;
                }
                // (BACKLINE_POP=1: the visuals in their own window too)
                if std::env::var_os("BACKLINE_POP").is_some() {
                    app.pop_toggle();
                }
            }
            // (BACKLINE_SONG=<file>: that song instead of the tour's)
            Ok("visuals") => {
                if let Ok(v) = std::env::var("BACKLINE_VIS") {
                    app.vis_preset = visuals::find(&v);
                }
                let song = std::env::var_os("BACKLINE_SONG").map(std::path::PathBuf::from).or_else(tour_song::ready);
                if let Some(p) = song {
                    let t = sources::ext_for_file(&p).to_track();
                    app.srcmap.insert(t.id, Src::File(p));
                    app.queue = Arc::new(vec![t]);
                    app.play_index(0);
                }
                app.vis_set_full(true, false);
            }
            // the New Playlist panel for the tour song (BACKLINE_ART=<picture file>: with that picture picked)
            Ok("newplaylist") => {
                let tracks = tour_song::ready().map(|p| sources::ext_for_file(&p).to_track()).into_iter().collect();
                app.open_new_playlist(new_playlist::NewPl {
                    tracks,
                    name: "Sunday morning".to_string(),
                    desc: "Slow ones to learn the heads to, before church.".to_string(),
                    art: std::env::var("BACKLINE_ART").unwrap_or_default(),
                    ..Default::default()
                });
            }
            Ok("listening") => {
                app.sec = Sec::Diary;
                app.diary_tab = 2;
            }
            Ok("plan") => {
                app.sec = Sec::Diary;
                app.diary_tab = 3;
            }
            Ok("analyzer") => {
                app.sec = Sec::Chords;
                app.chords_tab = 1;
                // an Em9 grip to show
                app.an_frets = [Some(0), Some(7), None, Some(12), Some(12), Some(7)];
            }
            Ok("lab") => {
                app.sec = Sec::Chords;
                app.chords_tab = 9;
                // a church walk in A-flat on the bench, its third chord picked
                if app.store.lab_bench.chords.is_empty() {
                    let c = |g: [Option<u8>; 6], beats: f32| crate::store::LabChord { grip: g, name: String::new(), beats, timed: true, art: 0 };
                    app.store.lab_bench = crate::store::LabProg {
                        name: "Church walk".into(),
                        bpm: 72.0,
                        chords: vec![
                            c([None, None, Some(6), Some(5), Some(4), Some(4)], 2.0),
                            c([None, None, Some(6), Some(6), Some(6), Some(4)], 1.5),
                            c([None, None, Some(6), Some(7), Some(6), Some(7)], 0.5),
                            c([None, Some(6), Some(6), Some(5), Some(4), Some(4)], 2.0),
                            // a single note walking down, then a rest
                            c([None, Some(3), None, None, None, None], 1.0),
                            c([None; 6], 1.0),
                        ],
                        link: None,
                    };
                }
                app.lab_sel = 2;
            }
            Ok("superimpose") => {
                app.sec = Sec::Lines;
                app.ln_tab = 3;
            }
            // BACKLINE_TP_MODE=1: through the changes
            Ok("triadpairs") => {
                app.sec = Sec::Lines;
                app.ln_tab = 6;
                app.tp_mode = std::env::var("BACKLINE_TP_MODE").ok().and_then(|s| s.parse::<u8>().ok()).unwrap_or(0).min(1);
            }
            Ok("fingerings") => {
                app.sec = Sec::Lines;
                app.ln_tab = 1;
                // BACKLINE_PF_SYS=<n>: that system (4 = ROOT UNDER A FINGER)
                if let Some(n) = std::env::var("BACKLINE_PF_SYS").ok().and_then(|s| s.parse::<u8>().ok()) {
                    app.pf_sys = n.min(4);
                }
            }
            Ok("melodies") => app.sec = Sec::Melodies,
            Ok("ear") => {
                app.sec = Sec::Ear;
                // BACKLINE_EAR=<drill>: a question already answered
                if let Some(d) = std::env::var("BACKLINE_EAR").ok().and_then(|s| s.parse::<usize>().ok()) {
                    app.ear_demo(d);
                }
            }
            Ok("sc") => app.sec = Sec::Sc,
            Ok("settings") => app.side = Some(Side::Backline),
            // the skins page (BACKLINE_WA_TAB=1 favorites, 2 recent); skinpreview: the skin worn, shown big
            Ok(w @ ("skins" | "skinpreview")) => {
                app.apply(Action::WaOpen);
                app.wa_tab = std::env::var("BACKLINE_WA_TAB").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
                // (BACKLINE_SKINS_SIDE=0 Backline's looks, 1 Winamp skins)
                if let Some(k) = std::env::var("BACKLINE_SKINS_SIDE").ok().and_then(|v| v.parse().ok()) {
                    app.skins_side = k;
                }
                if w == "skinpreview" {
                    app.wa_preview = app.wa_worn.clone().map(|(md5, name)| winamp::WaSkin {
                        shot: winamp::shot_url(&md5),
                        md5,
                        name,
                        download: String::new(),
                    });
                }
            }
            Ok("art") => app.art_view = true,
            Ok("timer") => {
                app.practice = true;
                app.timer_open = true;
            }
            Ok("metro") => {
                app.practice = true;
                app.metro_open = true;
            }
            Ok("chart") => {
                app.practice = true;
                app.sec = Sec::Chart;
                app.prac_chart = Some(("Cherokee".to_string(), crate::tour::CHEROKEE.to_string()));
            }
            Ok("tunes") => app.sec = Sec::Tunes,
            Ok("tunepage") => {
                app.sec = Sec::Tunes;
                app.tune_open = (!app.store.tunes.is_empty()).then_some(0);
            }
            Ok("reading") => app.sec = Sec::Reading,
            Ok("chords") => app.sec = Sec::Chords,
            Ok("lines") => {
                app.sec = Sec::Lines;
                // BACKLINE_LN_TAB=<tab number>: that LINES tab (5 = ATONAL)
                if let Some(t) = std::env::var("BACKLINE_LN_TAB").ok().and_then(|s| s.parse::<u8>().ok()) {
                    app.ln_tab = t.min(6);
                }
            }
            Ok("atonal") => {
                app.sec = Sec::Lines;
                app.ln_tab = 5;
                // BACKLINE_AT_VIEW=1: the generated lines
                app.at.view = std::env::var("BACKLINE_AT_VIEW").ok().and_then(|s| s.parse::<u8>().ok()).unwrap_or(0).min(1);
                // BACKLINE_AT_SEL=<note>: that note of your first line open to fix
                app.at.sel = std::env::var("BACKLINE_AT_SEL").ok().and_then(|s| s.parse::<usize>().ok());
            }
            Ok("files") => app.sec = Sec::Files,
            // a radio being saved as a playlist; one of your Tidal playlists being renamed; the note after a song is
            // put in a playlist (shown for its first seconds)
            Ok("saveradio") => {
                app.sec = Sec::Files;
                let tracks = (1..=40)
                    .map(|i| Track { id: 9000 + i, title: format!("Song {}", i), artist: "Doobie Powell".into(), ..Default::default() })
                    .collect();
                app.open_new_playlist(new_playlist::NewPl { name: "Walking uP radio".into(), tracks, ..Default::default() });
            }
            Ok("renamelist") => {
                app.sec = Sec::Files;
                app.apply(Action::TidalListRename("test".into(), "Shed tunes".into()));
            }
            Ok("toast") => {
                app.sec = Sec::Files;
                app.toast = Some(("Added to Walking uP radio".into(), 0.0));
            }
            // the other computer's songs (BACKLINE_PEER_NAME names it), some picked, some here, a copy going on; the
            // question when a playlist's song is there
            Ok("peerfiles") => {
                app.sec = Sec::Files;
                app.files_there = true;
                // (BACKLINE_FILES_HERE: this computer's songs in the list too)
                app.files_here = std::env::var_os("BACKLINE_FILES_HERE").is_some();
                let songs = [
                    ("Doobie Powell", "Walking uP", 3_920_000u64),
                    ("Doobie Powell", "I'm Blessed", 5_410_000),
                    ("Doobie Powell", "Peace & Love", 6_020_000),
                    ("Ari Hoenig", "For Tracy", 7_810_000),
                    ("J.A.M", "Quiet Wave", 4_480_000),
                    ("Ponta Box", "FELLINI", 4_900_000),
                    ("Kyoto Jazz Sextet", "Children Of Peace", 7_200_000),
                    ("Benedikt Jahnel Trio", "Mono Lake", 5_100_000),
                    ("Django Bates' Beloved", "Senza Bitterness", 4_700_000),
                    ("Avishai Cohen", "Lookie", 6_800_000),
                ];
                let list: Vec<sync::Shared> = songs
                    .iter()
                    .map(|(a, t, z)| sync::Shared { path: format!("D:/Music/{} - {}.mp3", a, t), title: t.to_string(), artist: a.to_string(), size: *z, ..Default::default() })
                    .collect();
                app.peer.here = [list[4].path.clone()].into_iter().collect();
                app.peer.sel = [list[0].path.clone(), list[1].path.clone(), list[3].path.clone()].into_iter().collect();
                app.peer.list = Some(Ok(list));
                let c = &app.copying;
                c.running.store(true, Ordering::Relaxed);
                *c.name.lock().unwrap() = "Peace & Love".into();
                c.size.store(6_020_000, Ordering::Relaxed);
                c.got.store(2_600_000, Ordering::Relaxed);
                c.done.store(2, Ordering::Relaxed);
                c.total.store(12, Ordering::Relaxed);
            }
            Ok("peerask") => {
                app.sec = Sec::Files;
                let t = Track { id: 9001, title: "Walking uP".into(), artist: "Doobie Powell".into(), ..Default::default() };
                app.peer_ask = Some((t, "D:/Music/Doobie Powell - Walking uP.mp3".into()));
            }
            // ADD TO PLAYLIST for a song
            Ok("addto") => {
                app.sec = Sec::Files;
                let t = Track { id: 9001, title: "Walking uP".into(), artist: "Doobie Powell".into(), ..Default::default() };
                app.apply(Action::PlaylistPicker(t, None));
            }
            // the question when a song is in the playlist already
            Ok("dupe") => {
                app.sec = Sec::Files;
                let t = Track { id: 9001, title: "Walking uP".into(), artist: "Doobie Powell".into(), ..Default::default() };
                app.pl_dupe = Some(new_playlist::PlDupe::Backline(0, "Quiet Wave radio".into(), t, None));
            }
            // a whole long list about to go in the queue; saving a queue that holds songs you added and a list
            Ok(w @ ("queueask" | "queuesave")) => {
                app.sec = Sec::Files;
                let t = |k: i64| Track { id: 9000 + k, title: format!("Song {}", k), ..Default::default() };
                let all: Vec<Track> = (0..5431).map(t).collect();
                if w == "queueask" {
                    app.queue_ask = Some(("THIS LIST".into(), all, false));
                } else {
                    app.queue_save = Some(new_playlist::QueueSave {
                        typed: String::new(),
                        from: "My Tracks".into(),
                        mine: all[..3].to_vec(),
                        all,
                    });
                }
            }
            Ok("slonimsky") => {
                app.sec = Sec::Lines;
                app.ln_tab = 0;
            }
            Ok("arpeggios") => {
                app.sec = Sec::Lines;
                app.ln_tab = 4;
            }
            Ok("voicings") => {
                app.sec = Sec::Chords;
                app.chords_tab = 3;
            }
            Ok("harmony") => {
                app.sec = Sec::Chords;
                app.chords_tab = 6;
            }
            Ok("pent") => {
                app.sec = Sec::Lines;
                app.ln_tab = 2;
            }
            Ok("diarysong") => {
                app.sec = Sec::Diary;
                if let Some((id, _)) =
                    app.store.days.iter().find(|d| d.date == store::today_str()).and_then(|d| d.songs.iter().next())
                {
                    app.diary_song = Some((store::today_str(), *id));
                }
            }
            Ok("libfilter") => {
                app.sec = Sec::Tidal;
                app.apply(Action::Library);
                app.lib_filter = "monk dream".to_string();
            }
            Ok("minefilter") => {
                // MY TRACKS filtered, the filter's text selected
                app.sec = Sec::Mine;
                app.lib_filter = "monk".to_string();
                app.ed.id = crate::tools::F_LIB_FILTER;
                app.ed.anchor = Some(0);
                app.ed.cur = 4;
            }
            Ok("scales") => {
                app.sec = Sec::Lines;
                app.ln_tab = 1;
                app.ln_sub = 3;
            }
            Ok("countoff") => {
                app.practice = true;
                app.more = true;
                app.mtab = 0;
                app.count_in = 4;
            }
            Ok("eq") => app.show_eq = true,
            Ok("buttonmap") => app.wa_map_until = 1e9,
            Ok("bookmarks") => {
                app.practice = true;
                app.more = true;
                app.mtab = 3;
            }
            Ok("triads") => {
                app.sec = Sec::Chords;
                app.chords_tab = 2;
            }
            Ok("cluster") => {
                app.sec = Sec::Chords;
                app.chords_tab = 4;
                app.cl_view = 1;
                app.fc_bass = 3;
            }
            Ok("group") => {
                app.sec = Sec::Loops;
                app.loops_mode = 2;
                app.grp_open = Some(String::new());
            }
            Ok("export") => {
                app.sec = Sec::Loops;
                app.share_view = Some((false, app.store.share()));
            }
            _ => {}
        }
    }
}

/// A test copy kept out of sight (a test profile without BACKLINE_SHOW): it never goes full screen or takes focus.
pub(crate) fn hidden_test_copy() -> bool {
    std::env::var_os("BACKLINE_HOME").is_some() && std::env::var_os("BACKLINE_SHOW").is_none()
}

/// BACKLINE_SHOT: once, after BACKLINE_SHOT_AT seconds (8 if not set), the app's own frame saved as a PNG there.
/// With BACKLINE_SHOTS=n, n frames BACKLINE_SHOT_EVERY seconds apart (name_000.png...), at most 960 wide: a test copy
/// filming itself (its own window only).
pub(crate) fn test_shot(ctx: &egui::Context) {
    static TAKEN: AtomicU32 = AtomicU32::new(0);
    static WAITING: AtomicBool = AtomicBool::new(false);
    shot(ctx, "BACKLINE_SHOT", &TAKEN, &WAITING);
}

/// A test copy's pictures of the window `ctx` is drawing, to the file the variable `var` names (as BACKLINE_SHOT),
/// counted in `taken`.
pub(crate) fn shot(
    ctx: &egui::Context,
    var: &str,
    taken: &AtomicU32,
    waiting: &AtomicBool,
) {
    let Ok(path) = std::env::var(var) else { return };
    let at: f64 = std::env::var("BACKLINE_SHOT_AT").ok().and_then(|s| s.parse().ok()).unwrap_or(8.0);
    let n: u32 = std::env::var("BACKLINE_SHOTS").ok().and_then(|s| s.parse().ok()).unwrap_or(1);
    let every: f64 = std::env::var("BACKLINE_SHOT_EVERY").ok().and_then(|s| s.parse().ok()).unwrap_or(0.1);
    let k = taken.load(Ordering::Relaxed);
    if k >= n {
        return;
    }
    if !waiting.load(Ordering::Relaxed) {
        if ctx.input(|i| i.time) > at + k as f64 * every {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
            waiting.store(true, Ordering::Relaxed);
        }
    } else {
        let shot = ctx.input(|i| {
            i.raw.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(img) = shot {
            let out = if n > 1 { path.replace(".png", &format!("_{:03}.png", k)) } else { path.clone() };
            std::thread::spawn(move || {
                let [w, h] = img.size;
                let px: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
                if let Some(buf) = image::RgbaImage::from_raw(w as u32, h as u32, px) {
                    let buf = if n > 1 && w > 960 {
                        image::imageops::resize(&buf, 960, (960 * h / w) as u32, image::imageops::FilterType::Triangle)
                    } else {
                        buf
                    };
                    let _ = buf.save(&out);
                }
            });
            taken.store(k + 1, Ordering::Relaxed);
            waiting.store(false, Ordering::Relaxed);
        }
    }
    ctx.request_repaint();
}

/// BACKLINE_LAB_TAPS=<seconds after the downbeat, comma-separated>: a test copy on the Progression Lab starts TAP
/// TIMING by itself 2 seconds in and taps at those times, to check it through without a hand on the keys.
pub(crate) fn lab_auto(app: &mut App, acts: &mut Vec<Action>) {
    use std::cell::Cell;
    thread_local! {
        static BEGAN: Cell<Option<Instant>> = const { Cell::new(None) };
        static STARTED: Cell<bool> = const { Cell::new(false) };
        static TAPPED: Cell<usize> = const { Cell::new(0) };
    }
    let Ok(v) = std::env::var("BACKLINE_LAB_TAPS") else { return };
    let times: Vec<f32> = v.split(',').filter_map(|s| s.trim().parse().ok()).collect();
    let began = BEGAN.with(|b| *b.get().get_or_insert_with(Instant::now));
    BEGAN.with(|b| b.set(Some(began)));
    if !STARTED.with(|s| s.get()) {
        if began.elapsed().as_secs_f32() >= 2.0 {
            STARTED.with(|s| s.set(true));
            app.lab_start_timing(acts);
        }
        return;
    }
    let Some(down) = app.lab_run.as_ref().and_then(|r| r.down) else { return };
    let k = TAPPED.with(|t| t.get());
    if let Some(at) = times.get(k) {
        if Instant::now() >= down + Duration::from_secs_f32(*at) {
            TAPPED.with(|t| t.set(k + 1));
            app.lab_tap();
        }
    }
}
