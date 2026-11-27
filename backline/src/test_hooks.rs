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
        // (BACKLINE_TIDAL_SAVE=<name>: your Backline playlist of that name made on your Tidal account too, as SAVE TO
        // TIDAL then Create on Tidal do; for your own copy, started by hand on your behalf: it makes no test copy, so
        // it keeps its sound. Once: gone from the surroundings, so a restart doesn't make it again)
        if let Ok(name) = std::env::var("BACKLINE_TIDAL_SAVE") {
            std::env::remove_var("BACKLINE_TIDAL_SAVE");
            match app.store.playlists.iter().find(|p| p.name == name) {
                Some(pl) if !pl.items.is_empty() && pl.items.iter().all(|v| v.kind == "tidal") && app.api.has_token() => {
                    let d = new_playlist::NewPl {
                        name: pl.name.clone(),
                        desc: pl.desc.clone(),
                        tracks: pl.items.iter().map(|v| v.to_track()).collect(),
                        tidal: true,
                        ..Default::default()
                    };
                    app.apply(Action::PlaylistMake(d));
                }
                _ => crate::api::log(&format!("BACKLINE_TIDAL_SAVE: no Backline playlist {:?} of Tidal songs, or not logged in", name)),
            }
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
                    // (BACKLINE_TWICE=1: the song queued twice, to hear one go into the next)
                    let n = if std::env::var_os("BACKLINE_TWICE").is_some() { 2 } else { 1 };
                    app.queue = Arc::new(vec![t; n]);
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
            // EDIT INFO open for the first YouTube song
            Ok("editinfo") => {
                app.sec = Sec::Yt;
                if let Some(e) = app.store.yt.first().cloned() {
                    app.apply(Action::EditInfo(e.id, e.title.clone(), e.artist.clone()));
                }
            }
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
                // (BACKLINE_NP_KEEP=1: the profile's own queue, not the tour song)
                let keep = std::env::var_os("BACKLINE_NP_KEEP").is_some();
                if let Some(p) = tour_song::ready().filter(|_| !keep) {
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
                // (BACKLINE_LOOP=<a>-<b>: with that loop set, for the practice view)
                if let Some((a, b)) = std::env::var("BACKLINE_LOOP").ok().and_then(|l| {
                    let (a, b) = l.split_once('-')?;
                    Some((a.parse::<f32>().ok()?, b.parse::<f32>().ok()?))
                }) {
                    (app.loop_a, app.loop_b) = (Some(a), Some(b));
                }
                // (BACKLINE_LYRICS=1: made-up synced lyrics for the song, for pictures of the lyrics)
                if let (Some(_), Some(t)) = (std::env::var_os("BACKLINE_LYRICS"), app.cur_track()) {
                    let words = ["The streetlights hum a minor key", "the last bus sighing up the hill", "I count the bars till you come home", "and play it slow, and play it still", "The radio is half asleep", "it hums along the way you do"];
                    let lines = (0..40).map(|i| (i as f32 * 3.0, words[i % words.len()].to_string())).collect();
                    app.lyrics = Some(Lyrics { id: t.id, status: 1, lines, synced: true });
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
            // the tour song playing, on the library (for pictures of the decks)
            Ok("playing") => {
                if let Some(p) = tour_song::ready() {
                    let mut t = sources::ext_for_file(&p).to_track();
                    // (BACKLINE_COVER=<picture file>: the song wears it as its cover)
                    if let Ok(c) = std::env::var("BACKLINE_COVER") {
                        t.cover = format!("img:{}", c);
                    }
                    app.srcmap.insert(t.id, Src::File(p.clone()));
                    let mut q = vec![t.clone()];
                    // (BACKLINE_QUEUE=n: n more songs after it, for pictures of Up Next)
                    let more = std::env::var("BACKLINE_QUEUE").ok().and_then(|v| v.parse::<usize>().ok()).unwrap_or(0);
                    for k in 0..more {
                        let mut c = t.clone();
                        c.id = t.id - 1 - k as i64;
                        c.title = format!("{} {}", t.title, k + 2);
                        app.srcmap.insert(c.id, Src::File(p.clone()));
                        q.push(c);
                    }
                    app.queue = Arc::new(q);
                    app.play_index(0);
                }
                // (BACKLINE_NP=1: in Now Playing; BACKLINE_HW_BIG=1: Hardware's close-up on the cover, or =0.5 halfway in;
                // BACKLINE_PRACTICE=1: practising)
                if std::env::var_os("BACKLINE_NP").is_some() {
                    app.art_view = true;
                }
                if std::env::var_os("BACKLINE_PRACTICE").is_some() {
                    app.practice = true;
                }
                if let Some(t) = std::env::var("BACKLINE_HW_BIG").ok().and_then(|v| v.parse::<f32>().ok()) {
                    app.hw_big = t >= 1.0;
                    app.hw_big_t = t;
                    app.hw_big_hold = t < 1.0;
                }
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
                // BACKLINE_PLAN_BLOCK=<n>: that block of today's session on
                if let Some(k) = std::env::var("BACKLINE_PLAN_BLOCK").ok().and_then(|s| s.parse::<usize>().ok()) {
                    app.plan.block = k;
                    app.plan.day = crate::store::today_days();
                }
            }
            Ok("analyzer") => {
                app.sec = Sec::Chords;
                app.chords_tab = 1;
                // an Em9 grip to show
                app.an_frets = [Some(0), Some(7), None, Some(12), Some(12), Some(7)];
            }
            Ok("chordbook") => {
                app.sec = Sec::Chords;
                app.chords_tab = 10;
                // BACKLINE_CB_SET / BACKLINE_CB_FAM: the string set and family to open (from 0)
                if let Some(v) = std::env::var("BACKLINE_CB_SET").ok().and_then(|v| v.parse().ok()) {
                    app.cb_set = v;
                }
                if let Some(v) = std::env::var("BACKLINE_CB_FAM").ok().and_then(|v| v.parse().ok()) {
                    app.cb_fam = v;
                }
                // BACKLINE_CB_MAP=1: the MAP
                if std::env::var("BACKLINE_CB_MAP").is_ok() {
                    app.cb_view = 4;
                }
                // BACKLINE_CB_CAD=<k>: 2-5-1, at chord k
                if let Some(k) = std::env::var("BACKLINE_CB_CAD").ok().and_then(|v| v.parse().ok()) {
                    app.cb_view = 3;
                    app.cb_cad_step = k;
                }
                // BACKLINE_CB_HOW=<k>: HOW IT WORKS, at step k of his chain
                if let Some(k) = std::env::var("BACKLINE_CB_HOW").ok().and_then(|v| v.parse().ok()) {
                    app.cb_view = 2;
                    app.cb_demo = k;
                }
                // BACKLINE_CB_WALK=<n>: WALK, at the n-th voicing (from 0)
                // BACKLINE_CB_ORDER=<0|1>: the lesson's order, or by station
                if let Some(v) = std::env::var("BACKLINE_CB_ORDER").ok().and_then(|v| v.parse::<u8>().ok()) {
                    app.cb_order = v.min(1);
                }
                if let Some(v) = std::env::var("BACKLINE_CB_WALK").ok().and_then(|v| v.parse().ok()) {
                    app.cb_view = 1;
                    app.cb_at = v;
                    // BACKLINE_CB_TRY=<k>: with the k-th rule tried on the neck
                    if let Some(k) = std::env::var("BACKLINE_CB_TRY").ok().and_then(|k| k.parse().ok()) {
                        app.cb_try = Some((v, k));
                    }
                }
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
                // BACKLINE_LAB_TAB=0..2: the tab open (Chords, Timing, Written Out)
                if let Some(t) = std::env::var("BACKLINE_LAB_TAB").ok().and_then(|v| v.parse::<u8>().ok()) {
                    app.lab_tab = t.min(2);
                }
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

/// BACKLINE_PRAC_DEMO=1: once the song is playing, a loop set around where it is, three saved loops (two overlapping)
/// and two named bookmarks, and the waveform zoomed to 24 s: for pictures of the practice waveform.
pub(crate) fn practice_demo(app: &mut App) {
    static DONE: AtomicBool = AtomicBool::new(false);
    if std::env::var_os("BACKLINE_PRAC_DEMO").is_none() || DONE.load(Ordering::Relaxed) {
        return;
    }
    let Some(id) = app.cur_track().map(|t| t.id) else { return };
    let pos = app.pos();
    if pos < 0.5 || app.track_len() < 60.0 {
        return;
    }
    DONE.store(true, Ordering::Relaxed);
    let t = pos + 4.0;
    app.loop_a = Some(t - 2.0);
    app.loop_b = Some(t + 6.5);
    app.loop_on = true;
    let sec = |name: &str, a: f32, b: f32| crate::store::Section { name: name.into(), a, b, ..Default::default() };
    app.store.sections.insert(id, vec![sec("Head", (t - 14.0).max(0.0), t - 4.0), sec("Chorus lick", t - 2.0, t + 6.5), sec("Turnaround", t + 3.0, t + 12.0)]);
    let mark = |name: &str, t: f32, col: [u8; 3]| crate::store::Bookmark { t, col, name: name.into(), ..Default::default() };
    app.store.marks.insert(id, vec![mark("Chorus", t - 6.0, [242, 193, 78]), mark("Lick", t + 9.0, [95, 180, 255])]);
    app.wv_zoom = (app.track_len() / 24.0).max(1.0);
    app.wv_start = (pos - 6.0).max(0.0);
    if let Ok(d) = std::env::var("BACKLINE_PRAC_DRAW") {
        app.wv_draw = d.parse().unwrap_or(0);
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
/// BACKLINE_LAB_LOOP=<a>,<b>[,play]: first the bench is linked to the song there, loop a to b, With the Loop picked
/// (then with "play" it plays over the loop instead; along the record the tap times count from the start).
pub(crate) fn lab_auto(app: &mut App, acts: &mut Vec<Action>) {
    use std::cell::Cell;
    thread_local! {
        static BEGAN: Cell<Option<Instant>> = const { Cell::new(None) };
        static STARTED: Cell<bool> = const { Cell::new(false) };
        static TAPPED: Cell<usize> = const { Cell::new(0) };
    }
    let looped = std::env::var("BACKLINE_LAB_LOOP").ok();
    let taps = std::env::var("BACKLINE_LAB_TAPS").ok();
    if looped.is_none() && taps.is_none() {
        return;
    }
    let times: Vec<f32> = taps.unwrap_or_default().split(',').filter_map(|s| s.trim().parse().ok()).collect();
    let began = BEGAN.with(|b| *b.get().get_or_insert_with(Instant::now));
    BEGAN.with(|b| b.set(Some(began)));
    if !STARTED.with(|s| s.get()) {
        if began.elapsed().as_secs_f32() >= 2.0 {
            STARTED.with(|s| s.set(true));
            if let Some(l) = looped {
                let parts: Vec<&str> = l.split(',').map(|x| x.trim()).collect();
                let num = |i: usize| parts.get(i).and_then(|x| x.parse::<f32>().ok()).unwrap_or(0.0);
                if let Some(t) = app.cur_track() {
                    app.remember_song(&t);
                    app.store.lab_bench.link = Some(crate::store::LabLink {
                        song: t.id,
                        title: t.title.clone(),
                        artist: t.artist.clone(),
                        a: num(0),
                        b: num(1),
                        name: "TEST LOOP".into(),
                        at: 0.0,
                    });
                    app.lab_with_loop = true;
                }
                if parts.get(2) == Some(&"play") {
                    app.lab_play_all(acts);
                    return;
                }
            }
            if !times.is_empty() {
                app.lab_start_timing(acts);
            }
        }
        return;
    }
    let Some(down) = app.lab_run.as_ref().map(|r| r.down.unwrap_or(r.start)) else { return };
    let k = TAPPED.with(|t| t.get());
    if let Some(at) = times.get(k) {
        if Instant::now() >= down + Duration::from_secs_f32(*at) {
            TAPPED.with(|t| t.set(k + 1));
            app.lab_tap();
        }
    }
}

/// A test copy's clicks (BACKLINE_CLICKS="x,y@t;x,y@t;..." in points, at seconds since it started): each a press and,
/// a frame later, a release at that place, put in the window's input, so a click can be checked in a picture.
pub(crate) fn fake_clicks(raw: &mut egui::RawInput) {
    use std::sync::Mutex;
    /// a click or a drag: from, to, when, Ctrl / Shift held, and how far along it is (one step a frame)
    struct Fake {
        a: egui::Pos2,
        b: egui::Pos2,
        t: f64,
        mods: egui::Modifiers,
        button: egui::PointerButton,
        hover: bool,
        /// a turn of the wheel there instead (points, down negative)
        wheel: f32,
        step: u8,
    }
    static LEFT: Mutex<Option<Vec<Fake>>> = Mutex::new(None);
    let mut left = LEFT.lock().unwrap_or_else(|e| e.into_inner());
    if left.is_none() {
        // "x,y@t" clicks; "x,y>x2,y2@t" drags; "x,y*dy@t" turns the wheel there; a leading ^ holds Ctrl, + holds
        // Shift, ~ is the right button, ? only moves the pointer there (a hover)
        let list = std::env::var("BACKLINE_CLICKS").unwrap_or_default();
        let pos = |xy: &str| -> Option<egui::Pos2> {
            let (x, y) = xy.split_once(',')?;
            Some(egui::Pos2::new(x.trim().parse().ok()?, y.trim().parse().ok()?))
        };
        *left = Some(
            list.split(';')
                .filter_map(|c| {
                    let (mut what, t) = c.trim().split_once('@')?;
                    let mut mods = egui::Modifiers::default();
                    let (mut button, mut hover) = (egui::PointerButton::Primary, false);
                    loop {
                        if let Some(r) = what.strip_prefix('^') {
                            (mods.ctrl, mods.command, what) = (true, true, r);
                        } else if let Some(r) = what.strip_prefix('+') {
                            (mods.shift, what) = (true, r);
                        } else if let Some(r) = what.strip_prefix('~') {
                            (button, what) = (egui::PointerButton::Secondary, r);
                        } else if let Some(r) = what.strip_prefix('?') {
                            (hover, what) = (true, r);
                        } else {
                            break;
                        }
                    }
                    let mut wheel = 0.0;
                    if let Some((xy, dy)) = what.split_once('*') {
                        (what, wheel) = (xy, dy.trim().parse().ok()?);
                    }
                    let (a, b) = match what.split_once('>') {
                        Some((a, b)) => (pos(a)?, pos(b)?),
                        None => (pos(what)?, pos(what)?),
                    };
                    Some(Fake { a, b, t: t.trim().parse().ok()?, mods, button, hover, wheel, step: 0 })
                })
                .collect(),
        );
    }
    let Some(now) = raw.time else { return };
    let Some(list) = left.as_mut() else { return };
    for c in list.iter_mut() {
        if c.step >= 5 || now < c.t {
            continue;
        }
        // press, three moves on the way (a drag only), release
        let drag = c.a != c.b;
        // (the first move a hand's few pixels, enough to count as a drag; then the rest of the way)
        let first = c.a + (c.b - c.a).normalized() * 8.0;
        let at = |k: f32| if k < 0.4 { first } else { first + (c.b - first) * k };
        raw.modifiers = c.mods;
        if c.wheel != 0.0 {
            raw.events.push(egui::Event::PointerMoved(c.a));
            raw.events.push(egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Point, delta: egui::Vec2::new(0.0, c.wheel), modifiers: c.mods });
            crate::api::log(&format!("test wheel {} at {:.0},{:.0}", c.wheel, c.a.x, c.a.y));
            c.step = 5;
            break;
        }
        if c.hover {
            raw.events.push(egui::Event::PointerMoved(c.a));
            c.step = 5;
            break;
        }
        match c.step {
            0 => {
                raw.events.push(egui::Event::PointerMoved(c.a));
                raw.events.push(egui::Event::PointerButton { pos: c.a, button: c.button, pressed: true, modifiers: c.mods });
                crate::api::log(&format!("test {} at {:.0},{:.0}", if drag { "drag" } else { "click" }, c.a.x, c.a.y));
                c.step = if drag { 1 } else { 4 };
            }
            1..=3 => {
                raw.events.push(egui::Event::PointerMoved(at(c.step as f32 / 3.0)));
                c.step += 1;
            }
            _ => {
                raw.events.push(egui::Event::PointerMoved(c.b));
                raw.events.push(egui::Event::PointerButton { pos: c.b, button: c.button, pressed: false, modifiers: c.mods });
                c.step = 5;
            }
        }
        break;
    }
}

// ------------------------------------------------------------------ the layout scan
/// BACKLINE_SCAN=1 (test copies): every half second, everything drawn is looked over for text cut off where it's
/// clipped, text drawn over other text, and words running out of the box round them; each found goes in the log once
/// as "layout: ..." (the sweep script collects them).
pub(crate) fn scan_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("BACKLINE_SCAN").is_some())
}

/// What the pixel font drew (as images, not text, so the scan can't see it otherwise): layer, clip, where, what.
static DOTS: std::sync::Mutex<Vec<(egui::LayerId, Rect, Rect, String)>> = std::sync::Mutex::new(Vec::new());
/// Clips that cut text on purpose (a line scrolling through its box).
static OK_CLIPS: std::sync::Mutex<Vec<Rect>> = std::sync::Mutex::new(Vec::new());

/// Pixel-font text drawn at `r` by painter `p` (only while scanning).
pub(crate) fn note_text(p: &egui::Painter, r: Rect, text: &str) {
    if scan_on() {
        DOTS.lock().unwrap_or_else(|e| e.into_inner()).push((p.layer_id(), p.clip_rect(), r, text.to_string()));
    }
}

/// A clip that cuts its text on purpose (a scrolling line): the scan lets it be.
pub(crate) fn clip_ok(r: Rect) {
    if scan_on() {
        OK_CLIPS.lock().unwrap_or_else(|e| e.into_inner()).push(r);
    }
}

struct Seen {
    layer: egui::LayerId,
    clip: Rect,
    r: Rect,
    text: String,
}

fn scan_walk(shape: &egui::Shape, clip: Rect, layer: egui::LayerId, texts: &mut Vec<Seen>, boxes: &mut Vec<(egui::LayerId, Rect)>) {
    match shape {
        egui::Shape::Vec(v) => {
            for s in v {
                scan_walk(s, clip, layer, texts, boxes);
            }
        }
        egui::Shape::Text(t) => {
            let text = t.galley.text().trim().to_string();
            let col = t.override_text_color.unwrap_or_else(|| {
                t.galley.job.sections.first().map(|s| s.format.color).filter(|c| *c != Color32::PLACEHOLDER).unwrap_or(t.fallback_color)
            });
            if text.is_empty() || t.angle != 0.0 || (col.a() as f32 * t.opacity_factor) < 90.0 {
                return;
            }
            // (the letters, not the line's full height: a little in from top and bottom)
            let r = shape.visual_bounding_rect();
            let r = r.shrink2(Vec2::new(0.0, r.height() * 0.18));
            texts.push(Seen { layer, clip, r, text });
        }
        // (a box only where it shows: one its clip hides, or mostly hides, isn't one the words can run out of)
        egui::Shape::Rect(rs) if rs.stroke.width > 0.0 && rs.stroke.color.a() > 60 => {
            let shown = rs.rect.intersect(clip);
            if shown.width() >= rs.rect.width() - 2.0 && shown.height() >= rs.rect.height() - 2.0 {
                boxes.push((layer, rs.rect));
            }
        }
        _ => {}
    }
}

/// Look over this frame's drawing (BACKLINE_SCAN): text cut by its clip, text over text, words out of their box.
pub(crate) fn scan_layout(ctx: &egui::Context) {
    if !scan_on() {
        return;
    }
    let dots: Vec<_> = std::mem::take(&mut *DOTS.lock().unwrap_or_else(|e| e.into_inner()));
    let ok: Vec<Rect> = std::mem::take(&mut *OK_CLIPS.lock().unwrap_or_else(|e| e.into_inner()));
    static LAST: std::sync::Mutex<f64> = std::sync::Mutex::new(0.0);
    let now = ctx.input(|i| i.time);
    {
        let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
        // (once it's settled, every half second)
        if now < 2.5 || now - *last < 0.5 {
            return;
        }
        *last = now;
    }
    let screen = ctx.screen_rect();
    let mut texts: Vec<Seen> = dots
        .into_iter()
        .filter(|d| !d.3.trim().is_empty())
        .map(|(layer, clip, r, text)| Seen { layer, clip, r, text: text.trim().to_string() })
        .collect();
    let mut boxes: Vec<(egui::LayerId, Rect)> = Vec::new();
    let layers: Vec<egui::LayerId> = ctx.memory(|m| m.layer_ids().collect());
    ctx.graphics(|g| {
        for l in layers {
            if let Some(list) = g.get(l) {
                for cs in list.all_entries() {
                    scan_walk(&cs.shape, cs.clip_rect, l, &mut texts, &mut boxes);
                }
            }
        }
    });
    let near = |a: Rect, b: Rect| (a.min - b.min).length() < 2.0 && (a.max - b.max).length() < 2.0;
    let mut found: Vec<String> = Vec::new();
    // (what shows of each: inside its clip and the window)
    let vis: Vec<Rect> = texts.iter().map(|t| t.r.intersect(t.clip).intersect(screen)).collect();
    for (i, t) in texts.iter().enumerate() {
        let v = vis[i];
        if v.width() <= 0.0 || v.height() < t.r.height() * 0.5 || t.r.width() < 6.0 {
            continue;
        }
        // cut: the words run past the side of their clip (or the window), so part of them doesn't show
        let clip = t.clip.intersect(screen);
        let over = (t.r.max.x - clip.max.x).max(clip.min.x - t.r.min.x);
        // (a line scrolling through its box is cut, and runs past it, on purpose)
        let scrolling = ok.iter().any(|o| near(*o, t.clip) || near(*o, clip));
        if over > 3.0 && !scrolling {
            found.push(format!("cut \"{}\" by {:.0} px at {:.0},{:.0}", t.text, over, t.r.min.x, t.r.min.y));
        }
        // over other words on the same layer (not the same words drawn twice a hair apart, for weight)
        for (j, u) in texts.iter().enumerate().skip(i + 1) {
            if u.layer != t.layer || (u.text == t.text && (u.r.min - t.r.min).length() < 3.0) {
                continue;
            }
            let w = vis[j];
            if w.width() <= 0.0 || w.height() <= 0.0 {
                continue;
            }
            let x = v.intersect(w);
            if x.width() > 3.0 && x.height() > 2.0 {
                found.push(format!("over \"{}\" and \"{}\" at {:.0},{:.0}", t.text, u.text, x.min.x, x.min.y));
            }
        }
        // out of its box: starts in a box (a button, a field, a tag) and runs out past its side (what shows of it)
        for (l, b) in boxes.iter().filter(|_| !scrolling) {
            if *l != t.layer || b.height() > 64.0 || b.width() < 12.0 {
                continue;
            }
            let cy = v.center().y;
            if cy < b.min.y + 2.0 || cy > b.max.y - 2.0 || t.r.height() > b.height() + 4.0 {
                continue;
            }
            let right = v.min.x >= b.min.x - 2.0 && v.min.x < b.max.x - 4.0 && v.max.x > b.max.x + 2.0;
            let left = v.max.x <= b.max.x + 2.0 && v.max.x > b.min.x + 4.0 && v.min.x < b.min.x - 2.0;
            if right || left {
                found.push(format!("out of its box \"{}\" at {:.0},{:.0} (box {:.0} wide)", t.text, t.r.min.x, t.r.min.y, b.width()));
            }
        }
    }
    static TOLD: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
    let mut told = TOLD.lock().unwrap_or_else(|e| e.into_inner());
    for f in found {
        // (each once: by what and roughly where)
        if told.len() < 400 && !told.contains(&f) {
            crate::api::log(&format!("layout: {}", f));
            told.push(f);
        }
    }
}
