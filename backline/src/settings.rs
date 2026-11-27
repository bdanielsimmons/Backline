//! Settings: what is kept between runs (settings.json in the app's folder), read at start (`load_settings`), put back
//! (`restore_settings`) and written as things change (`save_settings`). A new setting goes in both, here.

use super::*;

/// The settings saved last time (Null when there are none yet).
pub(crate) fn load_settings() -> serde_json::Value {
    std::fs::read_to_string(api::config_dir().join("settings.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or(serde_json::Value::Null)
}

/// Settings kept under their own field's name: read back when there (and of their type; else the field keeps what it
/// starts as), some kept in range (`=> clamp(..)`, `=> min(..)`), and written as they are.
macro_rules! plain_settings {
    ($( $f:ident: $t:ty $(=> $m:ident($($a:expr),*))? ),* $(,)?) => {
        impl App {
            fn restore_plain(&mut self, st: &serde_json::Value) {
                $( if let Ok(v) = serde_json::from_value::<$t>(st[stringify!($f)].clone()) { self.$f = v $(.$m($($a),*))?; } )*
            }
            fn save_plain(&self, m: &mut serde_json::Map<String, serde_json::Value>) {
                $( m.insert(stringify!($f).to_string(), serde_json::json!(self.$f)); )*
            }
        }
    };
}

plain_settings! {
    xfade: u8 => min(12),
    volume: f32 => clamp(0.0, 1.0),
    vis_idle: u32,
    vis_fps: u8,
    spec_op: f32 => clamp(0.05, 0.8),
    art_op: f32 => clamp(0.5, 1.0),
    wv_h: f32,
    mt_vol: f32,
    q_drop: bool,
    chart_lock: bool,
    lab_snap: u8 => min(3),
    lab_metro: bool,
    lab_with_loop: bool,
    lab_repeat: bool,
    lab_tab: u8 => min(2),
    cb_set: usize,
    cb_fam: usize,
    cb_view: u8 => min(4),
    cb_sound: bool,
    cb_order: u8 => min(1),
    cb_hide: bool,
    lab_vol: f32 => clamp(0.0, 1.0),
    lab_loop_chords: bool,
    neck_zoom: f32 => clamp(0.8, 2.2),
    ghost_alpha: f32 => clamp(0.1, 1.0),
    intro: u8 => min(2),
    auto_restart: bool,
    tour_seen: bool,
    cap_size: u8 => min(2),
    cap_dark: bool,
    cap_show: u8 => min(2),
    art_border: u8 => min(2),
    spec_lyr: u8 => min(1),
    spec_frame: u8 => min(1),
    spec_w: f32 => clamp(0.2, 1.0),
    spec_h: f32 => clamp(0.15, 0.9),
    eq_on: bool,
    keep_cache: bool,
    prac_stack_w: f32,
    nav_w: f32,
    stack_w: f32,
    lib_frac: f32 => clamp(0.2, 0.7),
    stack_frac: f32 => clamp(0.1, 1.0),
    chime_vol: f32 => clamp(0.0, 1.0),
    pomo_sound: bool,
    speed: u32 => clamp(25, 250),
    vis_np: bool,
    wv_follow: bool,
    wv_draw: u8 => min(2),
    seen_build: u64,
    tr_whole: bool,
    mine_stems: bool,
    eq_auto: bool,
    look_custom: bool,
    mini_viz: bool,
    np_tab: u8 => min(3),
    hw_show: u8 => min(5),
    hw_big: bool,
    studio_np: u8 => min(5),
    studio_vis: u8 => min(3),
    studio_terr_cam: u8 => min(2),
    studio_terr_mouse: u8 => min(3),
    studio_terr_swell: bool,
    style_col_w: f32,
    style_deck_h: f32,
    style_prac_col_w: f32,
    time_left: bool,
    wa_guide: bool,
    wa_outline: bool,
    diary_span: u8 => min(2),
    diary_reading: bool,
    loops_show: u8 => min(2),
    peer_play: u8 => min(2),
    upd_at: u64,
}

impl App {
    /// The settings saved last time, put back (at start, before the first frame).
    pub(crate) fn restore_settings(&mut self, st: &serde_json::Value) {
        let app = self;
        app.restore_plain(st);
        // always the best quality Tidal gives (it sends lossless when it will, else 320): no setting
        app.prefer_lossless = true;
        if let Some(v) = st["vis_preset"].as_str() {
            app.vis_preset = visuals::offered(visuals::find(v));
        }
        crate::retro::FPS.store(app.vis_fps, std::sync::atomic::Ordering::Relaxed);
        if let Some(a) = st["vis_pop_px"].as_array().filter(|a| a.len() == 4) {
            let v: Vec<i32> = a.iter().map(|x| x.as_i64().unwrap_or(0) as i32).collect();
            app.pop_px = Some([v[0], v[1], v[2], v[3]]);
        }
        app.pop_pin = st["vis_pop_pin"].as_bool().unwrap_or(true);
        if let Some(b) = st["spec"].as_bool() {
            app.show_spec = b;
        }
        // older saves had one shared set: both visualizers start from it
        for k in 0..2 {
            app.viz[k].load_old(st);
            if let Some(v) = st["viz"].get(k) {
                app.viz[k].load(v);
            }
        }
        if let Some(z) = st["zoom"].as_f64() {
            app.ctx.set_zoom_factor((z as f32).clamp(0.8, 2.0));
        }
        app.lib_zoom = st["lib_zoom"].as_f64().map_or(1.0, |z| (z as f32).clamp(0.8, 2.5));
        skin::LOOP_SAFE.store(st["loop_safe"].as_bool().unwrap_or(false), Ordering::Relaxed);
        skin::CALM.store(st["reduce_motion"].as_bool().unwrap_or(false), Ordering::Relaxed);
        // SIMPLE: settings from before it was a switch don't have it, so the copy starts as the edition it was built
        // as, and saves that, so the update after (one app for everyone) keeps it
        match st["simple"].as_bool() {
            Some(b) => crate::SIMPLE.store(b, Ordering::Relaxed),
            None => app.dirty = true,
        }
        app.wv_notes = practice_tools() && st["wv_notes"].as_bool().unwrap_or(false);
        if let Some(v) = st["export_pick"].as_u64() {
            app.export_pick = (v as usize).min(crate::extras::EXPORT_AS.len() - 1);
        }
        app.export_dir = st["export_dir"].as_str().filter(|d| !d.is_empty()).map(std::path::PathBuf::from);
        app.mark_col = serde_json::from_value(st["mark_col"].clone()).unwrap_or(None);
        // the first start of a new version: what changed (and in the few before it, for anyone who skipped some);
        // a first install (no settings yet) has nothing to catch up on
        let installed_before = st.as_object().is_some_and(|o| !o.is_empty());
        // (not over a demo being recorded or a test copy)
        let quiet = ["BACKLINE_DEMO", "BACKLINE_OPEN", "BACKLINE_SHOT"]
            .iter()
            .any(|v| std::env::var_os(v).is_some_and(|x| !x.is_empty()));
        if update::build() > 0 && app.seen_build != update::build() && !quiet {
            if installed_before {
                app.apply(Action::News);
            } else {
                app.seen_build = update::build();
            }
        }
        app.player.send(Cmd::MetroVolume(app.mt_vol * app.mt_vol));
        if let Some(l) = st["mine_off"].as_array() {
            app.mine_off = l.iter().filter_map(|v| v.as_str().map(String::from)).collect();
        }
        if let Some(v) = st["chord_sp"].as_array() {
            let b = |i: usize, d: bool| v.get(i).and_then(|x| x.as_bool()).unwrap_or(d);
            app.chord_sp =
                theory::Spelling { symbols: b(0, false), sharps: b(1, false), detailed: b(2, false), slash: b(3, true) };
        }
        app.intro_file = st["intro_file"].as_str().map(|s| s.to_string());
        if let Some(n) = st["intro_sound"].as_u64() {
            app.intro_sound = (n as usize).min(intro::SOUNDS.len() - 1) as u8;
        }
        if st.get("binds").is_some() {
            app.binds = prefs::binds_from_json(&st["binds"]);
        }
        // the very first run (no settings yet)
        app.first_look = !api::config_dir().join("settings.json").exists() && tour::demo_kind().is_none();
        // a new build since last time (an update): have Windows show its icon afresh
        if st["icon_build"].as_u64() != Some(update::build()) {
            media::refresh_icons();
            app.dirty = true;
        }
        // (a new key: everyone starts with the names hidden, the old "icon_names" is ignored)
        if let Some(b) = st["icon_words"].as_bool() {
            app.icon_names = b;
        }
        // watch the sound outputs: something plugged in or out, or a Bluetooth device coming or going, and Backline
        // moves with it (the list is always up to date)
        {
            let (tx, ctx) = (app.tx.clone(), app.ctx.clone());
            std::thread::spawn(move || {
                use rodio::cpal::traits::{DeviceTrait, HostTrait};
                let mut last = (String::new(), Vec::new());
                let mut first = true;
                loop {
                    let now = rodio::cpal::default_host().default_output_device().and_then(|d| d.name().ok()).unwrap_or_default();
                    let outs = crate::player::outputs();
                    let list: Vec<String> = outs.iter().map(|o| o.0.clone()).collect();
                    if first || now != last.0 || list != last.1 {
                        let bt = outs.iter().filter(|o| o.1).map(|o| o.0.clone()).collect();
                        if tx.send(Msg::Outputs(now.clone(), list.clone(), bt)).is_err() {
                            break;
                        }
                        ctx.request_repaint();
                        last = (now, list);
                        first = false;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(1200));
                }
            });
        }
        if let Some(d) = st["out_device"].as_str() {
            app.out_device = Some(d.to_string());
            app.player.send(Cmd::Device(app.out_device.clone()));
        }
        // the Winamp skin worn last time, from its saved file
        if let Some(list) = st["wa_favs"].as_array() {
            app.wa_favs = list.iter().filter_map(|v| Some((v[0].as_str()?.to_string(), v[1].as_str()?.to_string()))).collect();
        }
        if let Some(list) = st["wa_recent"].as_array() {
            app.wa_recent = list.iter().filter_map(|v| Some((v[0].as_str()?.to_string(), v[1].as_str()?.to_string()))).collect();
        }
        if let Some(b) = st["balance"].as_i64() {
            app.balance = (b as i32).clamp(-100, 100);
            app.player.ctl.set_balance(app.balance);
        }
        if let Some(f) = st["pl_font"].as_u64() {
            app.pl_font = (f as usize).min(winamp_ui::PL_FONTS.len() - 1);
        }
        // your Custom look's colors (its display and writing, when they're yours)
        let rgb = |v: &serde_json::Value| {
            let a = v.as_array()?;
            Some(Color32::from_rgb(a.first()?.as_u64()? as u8, a.get(1)?.as_u64()? as u8, a.get(2)?.as_u64()? as u8))
        };
        if let (Some(b), Some(a)) = (rgb(&st["custom_look"][0]), rgb(&st["custom_look"][1])) {
            app.custom = skin::CustomLook { background: b, accent: a, display: rgb(&st["custom_look"][2]), text: rgb(&st["custom_look"][3]) };
        }
        if let (Some(b), Some(a)) = (rgb(&st["ambient_look"][0]), rgb(&st["ambient_look"][1])) {
            app.ambient = skin::CustomLook { background: b, accent: a, display: None, text: None };
        }
        app.studio_terr_n = st["studio_terr_n"].as_u64().map_or(160, |v| v.clamp(16, 256) as usize);
        app.studio_terr_rows = st["studio_terr_rows2"].as_u64().map_or(160, |v| v.clamp(16, 240) as usize);
        // (the Chord Book always starts back in the main window: a restart puts a popped-out one back in)
        if let Some(a) = st["cb_pop_rect"].as_array().filter(|a| a.len() == 4) {
            let v: Vec<f32> = a.iter().filter_map(|x| x.as_f64()).map(|x| x as f32).collect();
            if v.len() == 4 && v[2] > 200.0 && v[3] > 200.0 {
                app.cb_pop_rect = Some([v[0], v[1], v[2], v[3]]);
            }
        }
        if let (Some(a), Some(b)) = (st["classic_worn"][0].as_str(), st["classic_worn"][1].as_str()) {
            app.classic_worn = Some((a.to_string(), b.to_string()));
        }
        for (i, at) in [
            &crate::skin::STREAM_ACC,
            &crate::skin::HW_SCREEN,
            &crate::skin::AMB_FEEL,
            &crate::skin::AMB_FADE,
            &crate::skin::STUDIO_SCREEN,
            &crate::skin::STUDIO_COVER,
            &crate::skin::HW_TYPE,
        ]
            .into_iter()
            .enumerate()
        {
            if let Some(v) = st["style_colors"][i].as_u64() {
                at.store(v as u8, Ordering::Relaxed);
            }
        }
        for (i, at) in [&crate::skin::STREAM_OWN, &crate::skin::HW_OWN[0], &crate::skin::HW_OWN[1], &crate::skin::STUDIO_OWN[0], &crate::skin::STUDIO_OWN[1]]
            .into_iter()
            .enumerate()
        {
            if let Some(v) = st["style_own"][i].as_u64() {
                at.store(v as u32 & 0x00ff_ffff, Ordering::Relaxed);
            }
        }        // the skin worn last time, put on before the first frame so the window opens already wearing it: one of
        // Backline's looks, your Custom one, or a Winamp skin (from its saved file; gone, Backline's look instead)
        let worn = (st["winamp"][0].as_str(), st["winamp"][1].as_str());
        let n = SKIN.load(Ordering::Relaxed) % LOOKS.len();
        match worn {
            (Some(looks::CUSTOM_MD5), _) => {
                let r = app.custom_skin();
                app.wear_skin(r, false);
            }
            (Some(looks::AMBIENT_MD5), _) => {
                let r = app.ambient_skin();
                app.wear_skin(r, false);
            }
            (Some(looks::STREAMING_MD5), _) => app.wear_skin(looks::style_skin(false), false),
            (Some(looks::HARDWARE_MD5), _) => app.wear_skin(looks::style_skin(true), false),
            (Some(looks::STUDIO_MD5), _) => app.wear_skin(looks::studio_skin(), false),
            (Some(md5), Some(name)) if !md5.starts_with("gen:") => {
                let s = winamp::WaSkin { md5: md5.to_string(), name: name.to_string(), shot: String::new(), download: String::new() };
                match winamp::load_saved(&s.md5) {
                    Some(b) => app.wear_skin(winamp::load(&b).map(|(p, art)| (s, p, art)), false),
                    None => {
                        let s = app.gen_skin(n);
                        app.wear_skin(genskin::wear(&PALS[n]).map(|(p, art)| (s, p, art)), false);
                    }
                }
            }
            _ => {
                let s = app.gen_skin(n);
                skin::set_mono(Some(n));
                app.wear_skin(genskin::wear(&PALS[n]).map(|(p, art)| (s, p, art)), false);
            }
        }
        // (a skin is always worn - the deck and the playlist are drawn from it: the one saved couldn't be put on,
        // Backline's own look instead)
        if app.wa_art.is_none() {
            let s = app.gen_skin(0);
            app.wear_skin(genskin::wear(&PALS[0]).map(|(p, art)| (s, p, art)), false);
        }
        if let Some(list) = st["eq_mine"].as_array() {
            for v in list {
                let name = v[0].as_str().unwrap_or_default().to_string();
                let g: Vec<f32> =
                    v[1].as_array().map(|a| a.iter().filter_map(|x| x.as_f64()).map(|x| x as f32).collect()).unwrap_or_default();
                if let (false, Ok(g)) = (name.is_empty(), <[f32; 10]>::try_from(g)) {
                    app.eq_mine.push((name, g));
                }
            }
        }
        if let Some(m) = st["eq_songs"].as_object() {
            for (k, v) in m {
                let g: Vec<f32> =
                    v.as_array().map(|a| a.iter().filter_map(|x| x.as_f64()).map(|x| x as f32).collect()).unwrap_or_default();
                if let (Ok(id), true) = (k.parse::<i64>(), g.len() == 10) {
                    app.eq_songs.insert(id, [g[0], g[1], g[2], g[3], g[4], g[5], g[6], g[7], g[8], g[9]]);
                }
            }
        }
        if let Some(b) = st["lyrics"].as_bool() {
            app.show_lyrics = b;
        }
        if let Some(r) = st["repeat"].as_u64() {
            app.repeat = match r {
                1 => Repeat::All,
                2 => Repeat::One,
                _ => Repeat::Off,
            };
        }
        if let Some(a) = st["eq"].as_array() {
            for (i, x) in a.iter().take(10).enumerate() {
                app.eq_gains[i] = (x.as_f64().unwrap_or(0.0) as f32).clamp(-12.0, 12.0);
            }
        }
        app.eq_pre = st["eq_pre"].as_f64().map_or(0.0, |v| (v as f32).clamp(-12.0, 12.0));
        // the hour your practice day ends (a late night stays on the day you started)
        if let Some(h) = st["day_ends"].as_u64() {
            crate::store::DAY_ENDS.store(h.min(6) as u8, std::sync::atomic::Ordering::Relaxed);
        }
        if let Some(b) = st["sc_browser"].as_u64() {
            sources::SC_BROWSER.store((b as u8).min(3), Ordering::Relaxed);
        }
        if st["offline"].as_bool() == Some(true) && !app.api.has_token() {
            app.offline = true;
            app.sec = Sec::Files;
        }
        app.eq_h_user = st["eq_h"].as_f64().map(|h| (h as f32).clamp(140.0, 900.0));
        app.pn_num = st["pn_num"].as_str().unwrap_or("").to_string();
        app.name_kept = st["name_kept"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        app.pf_num = st["pf_num"].as_str().unwrap_or("").to_string();
        app.ar_num = st["ar_num"].as_str().unwrap_or("").to_string();
        if let Some(a) = st["pf_stars"].as_array() {
            app.pf_stars = a.iter().filter_map(|v| v.as_str().map(String::from)).collect();
        }
        if st["go_offline"].as_bool() == Some(true) {
            app.go_offline = true;
            crate::api::OFFLINE.store(true, Ordering::Relaxed);
        }
        // (sorting by song is what LOOPS always does now: that old choice reads as A-Z)
        app.loops_sort = match st["loops_sort"].as_u64().unwrap_or(0) {
            3 => 1,
            s => s.min(2) as u8,
        };
        app.my_a = st["my_a"].as_f64().map(|a| a as f32).filter(|a| (380.0..=500.0).contains(a)).unwrap_or(440.0);
        app.eq_zoom = st["eq_zoom"].as_f64().map(|z| (z as f32).clamp(0.5, 8.0));
        app.deck_zoom = st["deck_zoom"].as_f64().map(|z| (z as f32).clamp(0.5, 8.0));
        // (a height you dragged; none: as tall as it can be. The old "prac_h" is left behind, so everyone starts tall)
        if let Some(h) = st["prac_h2"].as_f64() {
            app.prac_h = if h <= 0.0 { 0.0 } else { (h as f32).clamp(120.0, 900.0) };
        }
        if let Some(c) = st["cols"].as_u64() {
            // settings from before the KEY / BPM and SOURCE columns: shown once, they can be turned off from the header
            let v = st["cols_v"].as_u64().unwrap_or(1);
            let new = if v < 2 { 0b1000 } else { 0 } | if v < 3 { 0b10000 } else { 0 };
            COLS.store(c as u32 | new, Ordering::Relaxed);
        }
        if let Some(v) = st["track_sort"].as_u64() {
            crate::tracks::SORT.store(v as u8, Ordering::Relaxed);
        }
        if let Some(v) = st["vinyl"].as_bool() {
            VINYL.store(v, Ordering::Relaxed);
        }
        if let Some(v) = st["row_art"].as_bool() {
            ROW_ARTS.store(v, Ordering::Relaxed);
        }
        if let Some(w) = st["col_w"].as_array() {
            let w: Vec<f32> = w.iter().filter_map(|x| x.as_f64()).map(|x| x as f32).collect();
            if w.len() == 3 && w.iter().all(|x| *x > 0.01) {
                *COL_W.lock().unwrap() = [w[0], w[1], w[2]];
            }
        }
        if let Ok(t) = serde_json::from_value::<tools::TimerMore>(st["timer_more"].clone()) {
            app.tm = t;
        }
        // the focus timer as it was (closed, updated, crashed): held where it was, CONTINUE to carry on
        let num = |k: &str, lo: u64, hi: u64, d: u32| st[k].as_u64().map_or(d, |v| v.within(lo, hi) as u32);
        app.pomo_focus = num("pomo_focus", 1, 180, app.pomo_focus);
        app.pomo_break = num("pomo_break", 1, 60, app.pomo_break);
        app.pomo_cycles = num("pomo_cycles", 1, 16, app.pomo_cycles);
        let run = st["pomo_run"].as_u64().unwrap_or(0) as u8;
        let left = match run {
            1 | 2 => st["pomo_end_unix"].as_u64().unwrap_or(0).saturating_sub(unix_now()),
            4 => st["pomo_left"].as_u64().unwrap_or(0),
            _ => 0,
        };
        if left > 0 {
            app.pomo = 4;
            app.pomo_left = left;
            app.pomo_was = if run == 4 { st["pomo_was"].as_u64().unwrap_or(1) as u8 } else { run }.clamp(1, 2);
            app.pomo_n = num("pomo_n", 1, 16, 1);
        }
        if let Some(o) = st["loops"].as_object() {
            for (k, v) in o {
                if let (Ok(id), Some(a), Some(b)) = (k.parse::<i64>(), v[0].as_f64(), v[1].as_f64()) {
                    app.loops.insert(id, (a as f32, b as f32));
                }
            }
        }
    }

    pub(crate) fn save_settings(&mut self) {
        let mut v = serde_json::json!({
            "look_name": SKIN_NAMES[SKIN.load(Ordering::Relaxed) % LOOKS.len()],
            "classic_worn": self.classic_worn,
            "studio_terr_n": self.studio_terr_n,
            "studio_terr_rows2": self.studio_terr_rows,
            "cb_pop": self.cb_pop,
            "cb_pop_rect": self.cb_pop_rect,
            "style_colors": [
                crate::skin::STREAM_ACC.load(Ordering::Relaxed),
                crate::skin::HW_SCREEN.load(Ordering::Relaxed),
                crate::skin::AMB_FEEL.load(Ordering::Relaxed),
                crate::skin::AMB_FADE.load(Ordering::Relaxed),
                crate::skin::STUDIO_SCREEN.load(Ordering::Relaxed),
                crate::skin::STUDIO_COVER.load(Ordering::Relaxed),
                crate::skin::HW_TYPE.load(Ordering::Relaxed)
            ],
            // (your own colours for the styles: Streaming's accent, Hardware's writing and glass, Studio's glass and ink)
            "style_own": [
                crate::skin::STREAM_OWN.load(Ordering::Relaxed),
                crate::skin::HW_OWN[0].load(Ordering::Relaxed),
                crate::skin::HW_OWN[1].load(Ordering::Relaxed),
                crate::skin::STUDIO_OWN[0].load(Ordering::Relaxed),
                crate::skin::STUDIO_OWN[1].load(Ordering::Relaxed)
            ],
            "ambient_look": [
                [self.ambient.background.r(), self.ambient.background.g(), self.ambient.background.b()],
                [self.ambient.accent.r(), self.ambient.accent.g(), self.ambient.accent.b()]
            ],            "custom_look": [
                [self.custom.background.r(), self.custom.background.g(), self.custom.background.b()],
                [self.custom.accent.r(), self.custom.accent.g(), self.custom.accent.b()],
                self.custom.display.map(|c| vec![c.r(), c.g(), c.b()]),
                self.custom.text.map(|c| vec![c.r(), c.g(), c.b()])
            ],
            "spec": self.show_spec,
            "vis_preset": visuals::save_name(self.vis_preset),
            "vis_pop_px": self.pop_px,
            "vis_pop_pin": self.pop_pin,
            "icon_words": self.icon_names,
            "icon_build": update::build(),
            "mine_off": self.mine_off,
            "mark_col": self.mark_col,
            "export_pick": self.export_pick,
            "export_dir": self.export_dir.as_ref().map(|d| d.display().to_string()).unwrap_or_default(),
            "wv_notes": self.wv_notes,
            "chord_sp": [self.chord_sp.symbols, self.chord_sp.sharps, self.chord_sp.detailed, self.chord_sp.slash],
            "loop_safe": skin::LOOP_SAFE.load(Ordering::Relaxed),
            "reduce_motion": skin::calm(),
            "simple": !crate::practice_tools(),
            "intro_file": self.intro_file,
            "intro_sound": self.intro_sound,
            "zoom": self.ctx.zoom_factor(),
            "lib_zoom": self.lib_zoom,
            "binds": prefs::binds_to_json(&self.binds),
            "pl_font": self.pl_font,
            "eq_songs": self.eq_songs,
            "eq_mine": self.eq_mine,
            "balance": self.balance,
            "winamp": self.wa_worn,
            "wa_recent": self.wa_recent,
            "wa_favs": self.wa_favs,
            "out_device": self.out_device,
            "viz": [self.viz[0].save(), self.viz[1].save()],
            "lyrics": self.show_lyrics,
            "repeat": match self.repeat { Repeat::Off => 0, Repeat::All => 1, Repeat::One => 2 },
            "eq": self.eq_gains.to_vec(),
            "eq_pre": self.eq_pre,
            "day_ends": crate::store::DAY_ENDS.load(std::sync::atomic::Ordering::Relaxed),
            "offline": self.offline,
            "sc_browser": sources::SC_BROWSER.load(Ordering::Relaxed),
            "prac_h2": self.prac_h,
            "eq_h": self.eq_h_user,
            "deck_zoom": self.deck_zoom,
            "eq_zoom": self.eq_zoom,
            "go_offline": self.go_offline,
            "pn_num": self.pn_num,
            "name_kept": self.name_kept,
            "pf_num": self.pf_num,
            "ar_num": self.ar_num,
            "pf_stars": self.pf_stars,
            "loops_sort": self.loops_sort,
            "timer_more": serde_json::to_value(&self.tm).unwrap_or_default(),
            "pomo_focus": self.pomo_focus,
            "pomo_break": self.pomo_break,
            "pomo_cycles": self.pomo_cycles,
            "pomo_n": self.pomo_n,
            "pomo_run": if matches!(self.pomo, 1 | 2 | 4) { self.pomo } else { 0 },
            "pomo_end_unix": self.pomo_end.map_or(0, |e| unix_now() + e.saturating_duration_since(Instant::now()).as_secs()),
            "pomo_left": self.pomo_left,
            "pomo_was": self.pomo_was,
            "cols": COLS.load(Ordering::Relaxed),
            "track_sort": crate::tracks::SORT.load(Ordering::Relaxed),
            "cols_v": 3,
            "vinyl": VINYL.load(Ordering::Relaxed),
            "row_art": ROW_ARTS.load(Ordering::Relaxed),
            "my_a": self.my_a,
            "col_w": *COL_W.lock().unwrap(),
            "loops": self.loops.iter().map(|(k, v)| (k.to_string(), serde_json::json!([v.0, v.1]))).collect::<serde_json::Map<String, serde_json::Value>>(),
        });
        if let Some(m) = v.as_object_mut() {
            self.save_plain(m);
        }
        let dir = api::config_dir();
        let _ = std::fs::create_dir_all(&dir);
        // (written whole: a stop in the middle never leaves half a file)
        let tmp = dir.join("settings.json.tmp");
        if std::fs::write(&tmp, v.to_string()).is_ok() {
            let _ = std::fs::rename(&tmp, dir.join("settings.json"));
        }
        self.dirty = false;
        self.last_save = Instant::now();
    }
}
