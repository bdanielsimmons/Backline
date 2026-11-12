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

impl App {
    /// The settings saved last time, put back (at start, before the first frame).
    pub(crate) fn restore_settings(&mut self, st: &serde_json::Value) {
        let app = self;
        if let Some(v) = st["volume"].as_f64() {
            app.volume = (v as f32).clamp(0.0, 1.0);
        }
        // always the best quality Tidal gives (it sends lossless when it will, else 320): no setting
        app.prefer_lossless = true;
        app.vis_np = st["vis_np"].as_bool().unwrap_or(false);
        if let Some(v) = st["vis_preset"].as_str() {
            app.vis_preset = visuals::offered(visuals::find(v));
        }
        if let Some(m) = st["vis_idle"].as_u64() {
            app.vis_idle = m as u32;
        }
        if let Some(a) = st["vis_pop_px"].as_array().filter(|a| a.len() == 4) {
            let v: Vec<i32> = a.iter().map(|x| x.as_i64().unwrap_or(0) as i32).collect();
            app.pop_px = Some([v[0], v[1], v[2], v[3]]);
        }
        app.pop_pin = st["vis_pop_pin"].as_bool().unwrap_or(true);
        if let Some(b) = st["spec"].as_bool() {
            app.show_spec = b;
        }
        if let Some(f) = st["spec_op"].as_f64() {
            app.spec_op = (f as f32).clamp(0.05, 0.8);
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
        if let Some(f) = st["art_op"].as_f64() {
            app.art_op = (f as f32).clamp(0.5, 1.0);
        }
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
        if let Some(v) = st["wv_h"].as_f64() {
            app.wv_h = v as f32;
        }
        app.seen_build = st["seen_build"].as_u64().unwrap_or(0);
        app.tr_whole = st["tr_whole"].as_bool().unwrap_or(true);
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
        if let Some(v) = st["mt_vol"].as_f64() {
            app.mt_vol = v as f32;
        }
        app.player.send(Cmd::MetroVolume(app.mt_vol * app.mt_vol));
        if let Some(l) = st["mine_off"].as_array() {
            app.mine_off = l.iter().filter_map(|v| v.as_str().map(String::from)).collect();
        }
        if let Some(b) = st["q_drop"].as_bool() {
            app.q_drop = b;
        }
        if let Some(b) = st["chart_lock"].as_bool() {
            app.chart_lock = b;
        }
        app.mine_stems = st["mine_stems"].as_bool().unwrap_or(false);
        if let Some(v) = st["lab_snap"].as_u64() {
            app.lab_snap = (v as u8).min(3);
        }
        if let Some(v) = st["lab_metro"].as_bool() {
            app.lab_metro = v;
        }
        if let Some(v) = st["lab_with_loop"].as_bool() {
            app.lab_with_loop = v;
        }
        if let Some(v) = st["lab_repeat"].as_bool() {
            app.lab_repeat = v;
        }
        if let Some(v) = st["lab_written"].as_bool() {
            app.lab_written = v;
        }
        if let Some(z) = st["neck_zoom"].as_f64() {
            app.neck_zoom = (z as f32).clamp(0.8, 2.2);
        }
        if let Some(a) = st["ghost_alpha"].as_f64() {
            app.ghost_alpha = (a as f32).clamp(0.1, 1.0);
        }
        if let Some(v) = st["chord_sp"].as_array() {
            let b = |i: usize, d: bool| v.get(i).and_then(|x| x.as_bool()).unwrap_or(d);
            app.chord_sp =
                theory::Spelling { symbols: b(0, false), sharps: b(1, false), detailed: b(2, false), slash: b(3, true) };
        }
        if let Some(m) = st["intro"].as_u64() {
            app.intro = (m as u8).min(2);
        }
        app.intro_file = st["intro_file"].as_str().map(|s| s.to_string());
        if let Some(n) = st["intro_sound"].as_u64() {
            app.intro_sound = (n as usize).min(intro::SOUNDS.len() - 1) as u8;
        }
        if let Some(b) = st["auto_restart"].as_bool() {
            app.auto_restart = b;
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
        if let Some(b) = st["tour_seen"].as_bool() {
            app.tour_seen = b;
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
        if let Some(b) = st["cap_size"].as_u64() {
            app.cap_size = (b as u8).min(2);
        }
        if let Some(b) = st["cap_dark"].as_bool() {
            app.cap_dark = b;
        }
        if let Some(b) = st["cap_show"].as_u64() {
            app.cap_show = (b as u8).min(2);
        }
        if let Some(b) = st["art_border"].as_u64() {
            app.art_border = (b as u8).min(2);
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
        app.eq_auto = st["eq_auto"].as_bool().unwrap_or(false);
        // your Custom look's colors (its display and writing, when they're yours)
        let rgb = |v: &serde_json::Value| {
            let a = v.as_array()?;
            Some(Color32::from_rgb(a.first()?.as_u64()? as u8, a.get(1)?.as_u64()? as u8, a.get(2)?.as_u64()? as u8))
        };
        if let (Some(b), Some(a)) = (rgb(&st["custom_look"][0]), rgb(&st["custom_look"][1])) {
            app.custom = skin::CustomLook { background: b, accent: a, display: rgb(&st["custom_look"][2]), text: rgb(&st["custom_look"][3]) };
        }
        app.look_custom = st["look_custom"].as_bool().unwrap_or(false);        // the skin worn last time, put on before the first frame so the window opens already wearing it: one of
        // Backline's looks, your Custom one, or a Winamp skin (from its saved file; gone, Backline's look instead)
        let worn = (st["winamp"][0].as_str(), st["winamp"][1].as_str());
        let n = SKIN.load(Ordering::Relaxed) % LOOKS.len();
        match worn {
            (Some(looks::CUSTOM_MD5), _) => {
                let r = app.custom_skin();
                app.wear_skin(r, false);
            }
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

        if let Some(f) = st["spec_lyr"].as_u64() {
            app.spec_lyr = (f as u8).min(1);
        }
        if let Some(f) = st["spec_frame"].as_u64() {
            app.spec_frame = (f as u8).min(1);
        }
        if let Some(f) = st["spec_w"].as_f64() {
            app.spec_w = (f as f32).clamp(0.2, 1.0);
        }
        if let Some(f) = st["spec_h"].as_f64() {
            app.spec_h = (f as f32).clamp(0.15, 0.9);
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
        if let Some(b) = st["eq_on"].as_bool() {
            app.eq_on = b;
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
        if let Some(b) = st["keep_cache"].as_bool() {
            app.keep_cache = b;
        }
        if let Some(b) = st["sc_browser"].as_u64() {
            sources::SC_BROWSER.store((b as u8).min(3), Ordering::Relaxed);
        }
        if st["offline"].as_bool() == Some(true) && !app.api.has_token() {
            app.offline = true;
            app.sec = Sec::Files;
        }
        if let Some(f) = st["prac_stack_w"].as_f64() {
            app.prac_stack_w = f as f32;
        }
        if let Some(f) = st["nav_w"].as_f64() {
            app.nav_w = f as f32;
        }
        if let Some(f) = st["stack_w"].as_f64() {
            app.stack_w = f as f32;
        }
        if let Some(f) = st["lib_frac"].as_f64() {
            app.lib_frac = (f as f32).clamp(0.2, 0.7);
        }
        if let Some(f) = st["stack_frac"].as_f64() {
            app.stack_frac = (f as f32).clamp(0.1, 1.0);
        }
        app.eq_h_user = st["eq_h"].as_f64().map(|h| (h as f32).clamp(140.0, 900.0));
        app.wa_guide = st["wa_guide"].as_bool().unwrap_or(false);
        app.wa_outline = st["wa_outline"].as_bool().unwrap_or(false);
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
        app.diary_span = st["diary_span"].as_u64().unwrap_or(0).min(2) as u8;
        app.diary_reading = st["diary_reading"].as_bool().unwrap_or(true);
        // (sorting by song is what LOOPS always does now: that old choice reads as A-Z)
        app.loops_sort = match st["loops_sort"].as_u64().unwrap_or(0) {
            3 => 1,
            s => s.min(2) as u8,
        };
        app.loops_show = st["loops_show"].as_u64().unwrap_or(0).min(2) as u8;
        app.peer_play = st["peer_play"].as_u64().unwrap_or(0).min(2) as u8;
        app.upd_at = st["upd_at"].as_u64().unwrap_or(0);
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
        if let Some(b) = st["pomo_sound"].as_bool() {
            app.pomo_sound = b;
        }
        // the focus timer as it was (closed, updated, crashed): held where it was, CONTINUE to carry on
        let num = |k: &str, lo: u64, hi: u64, d: u32| st[k].as_u64().map_or(d, |v| v.clamp(lo, hi) as u32);
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
        if let Some(p) = st["speed"].as_u64() {
            app.speed = (p as u32).clamp(25, 250);
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
        let v = serde_json::json!({
            "look_name": SKIN_NAMES[SKIN.load(Ordering::Relaxed) % LOOKS.len()],
            "look_custom": self.look_custom,            "custom_look": [
                [self.custom.background.r(), self.custom.background.g(), self.custom.background.b()],
                [self.custom.accent.r(), self.custom.accent.g(), self.custom.accent.b()],
                self.custom.display.map(|c| vec![c.r(), c.g(), c.b()]),
                self.custom.text.map(|c| vec![c.r(), c.g(), c.b()])
            ],
            "volume": self.volume,
            "spec": self.show_spec,
            "vis_preset": visuals::save_name(self.vis_preset),
            "vis_np": self.vis_np,
            "vis_idle": self.vis_idle,
            "vis_pop_px": self.pop_px,
            "vis_pop_pin": self.pop_pin,
            "tour_seen": self.tour_seen,
            "icon_words": self.icon_names,
            "icon_build": update::build(),
            "auto_restart": self.auto_restart,
            "intro": self.intro,
            "neck_zoom": self.neck_zoom,
            "lab_snap": self.lab_snap,
            "lab_metro": self.lab_metro,
            "lab_with_loop": self.lab_with_loop,
            "lab_repeat": self.lab_repeat,
            "lab_written": self.lab_written,
            "q_drop": self.q_drop,
            "mine_off": self.mine_off,
            "mine_stems": self.mine_stems,
            "mt_vol": self.mt_vol,
            "seen_build": self.seen_build,
            "tr_whole": self.tr_whole,
            "mark_col": self.mark_col,
            "wv_h": self.wv_h,
            "export_pick": self.export_pick,
            "export_dir": self.export_dir.as_ref().map(|d| d.display().to_string()).unwrap_or_default(),
            "wv_notes": self.wv_notes,
            "chart_lock": self.chart_lock,
            "ghost_alpha": self.ghost_alpha,
            "chord_sp": [self.chord_sp.symbols, self.chord_sp.sharps, self.chord_sp.detailed, self.chord_sp.slash],
            "loop_safe": skin::LOOP_SAFE.load(Ordering::Relaxed),
            "reduce_motion": skin::calm(),
            "simple": !crate::practice_tools(),
            "intro_file": self.intro_file,
            "intro_sound": self.intro_sound,
            "art_op": self.art_op,
            "zoom": self.ctx.zoom_factor(),
            "lib_zoom": self.lib_zoom,
            "binds": prefs::binds_to_json(&self.binds),
            "spec_op": self.spec_op,
            "spec_h": self.spec_h,
            "spec_w": self.spec_w,
            "spec_frame": self.spec_frame,
            "spec_lyr": self.spec_lyr,
            "pl_font": self.pl_font,
            "eq_auto": self.eq_auto,
            "eq_songs": self.eq_songs,
            "eq_mine": self.eq_mine,
            "balance": self.balance,
            "winamp": self.wa_worn,
            "wa_recent": self.wa_recent,
            "wa_favs": self.wa_favs,
            "art_border": self.art_border,
            "cap_size": self.cap_size,
            "cap_dark": self.cap_dark,
            "cap_show": self.cap_show,
            "out_device": self.out_device,
            "viz": [self.viz[0].save(), self.viz[1].save()],
            "lyrics": self.show_lyrics,
            "repeat": match self.repeat { Repeat::Off => 0, Repeat::All => 1, Repeat::One => 2 },
            "eq_on": self.eq_on,
            "eq": self.eq_gains.to_vec(),
            "eq_pre": self.eq_pre,
            "keep_cache": self.keep_cache,
            "day_ends": crate::store::DAY_ENDS.load(std::sync::atomic::Ordering::Relaxed),
            "speed": self.speed,
            "lib_frac": self.lib_frac,
            "stack_w": self.stack_w,
            "prac_stack_w": self.prac_stack_w,
            "nav_w": self.nav_w,
            "offline": self.offline,
            "sc_browser": sources::SC_BROWSER.load(Ordering::Relaxed),
            "stack_frac": self.stack_frac,
            "prac_h2": self.prac_h,
            "eq_h": self.eq_h_user,
            "deck_zoom": self.deck_zoom,
            "eq_zoom": self.eq_zoom,
            "wa_guide": self.wa_guide,
            "wa_outline": self.wa_outline,
            "go_offline": self.go_offline,
            "pn_num": self.pn_num,
            "name_kept": self.name_kept,
            "pf_num": self.pf_num,
            "ar_num": self.ar_num,
            "pf_stars": self.pf_stars,
            "diary_span": self.diary_span,
            "diary_reading": self.diary_reading,
            "loops_sort": self.loops_sort,
            "loops_show": self.loops_show,
            "pomo_sound": self.pomo_sound,
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
            "cols_v": 3,
            "vinyl": VINYL.load(Ordering::Relaxed),
            "row_art": ROW_ARTS.load(Ordering::Relaxed),
            "my_a": self.my_a,
            "upd_at": self.upd_at,
            "peer_play": self.peer_play,
            "col_w": *COL_W.lock().unwrap(),
            "loops": self.loops.iter().map(|(k, v)| (k.to_string(), serde_json::json!([v.0, v.1]))).collect::<serde_json::Map<String, serde_json::Value>>(),
        });
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
