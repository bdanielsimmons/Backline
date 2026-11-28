//! RECORD: recording yourself over the song, as the desktop's Record tab does it (REC, the time, the level, the
//! sound coming in, Keep the last take?, the song's takes). On a phone the mic hears you and the song from its
//! speaker together, so a take is that: what you'd want to listen back to. The input is the desktop's own
//! (audio/record.rs); takes are WAV files under takes/, listed in takes.json.

use crate::*;

/// A take: which song, when, how long, its file (under takes/).
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct Take {
    pub vid: String,
    pub stamp: String,
    pub secs: f32,
    pub file: String,
}

fn takes_dir() -> std::path::PathBuf {
    api::config_dir().join("takes")
}

/// Mono 16-bit samples as a WAV file.
fn write_wav(path: &std::path::Path, rate: u32, s: &[i16]) -> std::io::Result<()> {
    let data = (s.len() * 2) as u32;
    let mut b: Vec<u8> = Vec::with_capacity(44 + s.len() * 2);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&rate.to_le_bytes());
    b.extend_from_slice(&(rate * 2).to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data.to_le_bytes());
    for v in s {
        b.extend_from_slice(&v.to_le_bytes());
    }
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d)?;
    }
    std::fs::write(path, b)
}

/// A WAV written by `write_wav`, back as mono 44.1 kHz samples.
fn read_wav(path: &std::path::Path) -> Option<Vec<i16>> {
    let b = std::fs::read(path).ok()?;
    let body = b.get(44..)?;
    Some(body.chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]])).collect())
}

/// The clock now as "Nov 28 05:12" (local time is the phone's own: UTC offset from the environment isn't known, so
/// the take's order is what counts; the stamp reads as the phone's clock where TZ is set).
fn stamp() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let off = local_offset();
    let t = secs + off;
    let (h, m) = ((t.rem_euclid(86400)) / 3600, (t.rem_euclid(3600)) / 60);
    let (_, mo, d) = civil(t.div_euclid(86400));
    const M: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    format!("{} {} {:02}:{:02}", M[(mo as usize + 11) % 12], d, h, m)
}

/// Days since 1970-01-01 to the year, month (1-12) and day (Howard Hinnant's civil-from-days).
pub(crate) fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + if mo <= 2 { 1 } else { 0 }, mo as u32, d as u32)
}

/// The phone's offset from UTC in seconds (from Android's own time zone, through Java).
pub(crate) fn local_offset() -> i64 {
    #[cfg(target_os = "android")]
    {
        let r = (|| -> Option<i64> {
            let ctx = ndk_context::android_context();
            let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }.ok()?;
            let mut env = vm.attach_current_thread().ok()?;
            let tz = env.call_static_method("java/util/TimeZone", "getDefault", "()Ljava/util/TimeZone;", &[]).ok()?.l().ok()?;
            let now = env.call_static_method("java/lang/System", "currentTimeMillis", "()J", &[]).ok()?.j().ok()?;
            let off = env.call_method(&tz, "getOffset", "(J)I", &[jni::objects::JValue::Long(now)]).ok()?.i().ok()?;
            Some(off as i64 / 1000)
        })();
        return r.unwrap_or(0);
    }
    #[allow(unreachable_code)]
    0
}

/// Whether the app may use the mic (asked for once with `ask_mic`).
pub(crate) fn mic_allowed() -> bool {
    #[cfg(target_os = "android")]
    {
        let r = (|| -> Option<bool> {
            let ctx = ndk_context::android_context();
            let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }.ok()?;
            let mut env = vm.attach_current_thread().ok()?;
            let activity = unsafe { jni::objects::JObject::from_raw(ctx.context().cast()) };
            let p = env.new_string("android.permission.RECORD_AUDIO").ok()?;
            let v = env.call_method(&activity, "checkSelfPermission", "(Ljava/lang/String;)I", &[(&p).into()]).ok()?.i().ok()?;
            Some(v == 0)
        })();
        return r.unwrap_or(false);
    }
    #[allow(unreachable_code)]
    true
}

/// Android's question: may Backline use the mic?
fn ask_mic() {
    #[cfg(target_os = "android")]
    {
        let _ = (|| -> Option<()> {
            let ctx = ndk_context::android_context();
            let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }.ok()?;
            let mut env = vm.attach_current_thread().ok()?;
            // (the Activity itself: the context the app is given is the Application, which can't ask)
            let act = crate::ANDROID.get()?.activity_as_ptr();
            let activity = unsafe { jni::objects::JObject::from_raw(act.cast()) };
            let p = env.new_string("android.permission.RECORD_AUDIO").ok()?;
            let arr = env.new_object_array(1, "java/lang/String", &p).ok()?;
            if let Err(e) = env.call_method(&activity, "requestPermissions", "([Ljava/lang/String;I)V", &[(&arr).into(), jni::objects::JValue::Int(7)]) {
                api::log(&format!("asking for the mic: {}", e));
                if env.exception_check().unwrap_or(false) {
                    let _ = env.exception_describe();
                    let _ = env.exception_clear();
                }
                return None;
            }
            api::log("asked for the mic");
            Some(())
        })();
    }
}

impl App {
    /// REC: the mic on (asked for the first time), the song playing; again: the take ends and is offered to keep.
    pub(crate) fn rec_toggle(&mut self) {
        if self.rec_buf.is_some() {
            self.rec_stop();
            return;
        }
        if !mic_allowed() {
            self.rec_waiting = true;
            ask_mic();
            return;
        }
        self.rec_go();
    }

    fn rec_go(&mut self) {
        self.rec_waiting = false;
        self.take_playing = None;
        self.player.send(Cmd::StopOnce);
        self.rec_buf = Some(record::start(None));
        self.rec_t0 = Some(Instant::now());
        self.rec_ask = None;
        // the song plays as the take starts (from the loop's start while looping)
        if self.cur.is_some() && (self.stopped || self.paused) && !self.loading {
            self.want_play = true;
            self.play_pause();
        }
    }

    fn rec_stop(&mut self) {
        record::stop();
        let Some(buf) = self.rec_buf.take() else { return };
        self.rec_t0 = None;
        let (rate, samples) = match buf.lock() {
            Ok(b) => (b.rate, b.samples.clone()),
            Err(_) => return,
        };
        let s = record::resample(&samples, rate, 44100);
        if s.len() < 44100 / 2 {
            self.set_note("That take was too short to keep");
            return;
        }
        let Some(cur) = self.cur.clone() else { return };
        let st = stamp();
        let file = format!("{}_{}.wav", cur.vid, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0));
        if let Err(e) = write_wav(&takes_dir().join(&file), 44100, &s) {
            self.set_note(&format!("Couldn't save the take: {}", e));
            return;
        }
        self.takes.push(Take { vid: cur.vid, stamp: st, secs: s.len() as f32 / 44100.0, file });
        save_json("takes.json", &self.takes);
        self.rec_ask = Some(self.takes.len() - 1);
    }

    /// A take heard (the song paused while it plays), or stopped.
    fn play_take(&mut self, i: usize) {
        self.player.send(Cmd::StopOnce);
        if self.take_playing.map(|(k, _)| k) == Some(i) {
            self.take_playing = None;
            return;
        }
        let Some(t) = self.takes.get(i) else { return };
        match read_wav(&takes_dir().join(&t.file)) {
            Some(s) => {
                if !self.stopped && !self.paused {
                    self.play_pause();
                }
                self.player.send(Cmd::Once(s, 1.0));
                self.take_playing = Some((i, Instant::now()));
            }
            None => self.set_note("That take's file is missing"),
        }
    }

    fn delete_take(&mut self, i: usize) {
        if i < self.takes.len() {
            let t = self.takes.remove(i);
            let _ = std::fs::remove_file(takes_dir().join(&t.file));
            save_json("takes.json", &self.takes);
        }
        self.take_playing = None;
        self.take_del = None;
        self.rec_ask = None;
    }

    /// Once a frame: the mic, once Android says yes; the take heard comes to its end.
    pub(crate) fn rec_tick(&mut self) {
        if self.rec_waiting {
            let now = self.ctx.input(|i| i.time);
            if now - self.rec_asked_at > 0.5 {
                self.rec_asked_at = now;
                if mic_allowed() {
                    self.rec_go();
                }
            }
            self.ctx.request_repaint_after(Duration::from_millis(500));
        }
        if let Some((i, at)) = self.take_playing {
            let len = self.takes.get(i).map_or(0.0, |t| t.secs);
            if at.elapsed().as_secs_f32() > len + 0.3 {
                self.take_playing = None;
            } else {
                self.ctx.request_repaint_after(Duration::from_millis(250));
            }
        }
    }

    /// RECORD, as the desktop's tab: REC, the time and the level; Keep the last take?; the sound coming in; the
    /// song's takes.
    pub(crate) fn record_tab(&mut self, ui: &mut egui::Ui) {
        let on = self.rec_buf.is_some();
        let red = Color32::from_rgb(230, 80, 70);
        centered_row(ui, "rec_1", |ui| {
            // REC: a dot and the word, red and blinking while the take runs
            let (r, resp) = ui.allocate_exact_size(Vec2::new(86.0 * font::ui_scale(), bh()), Sense::click());
            let p = ui.painter();
            raised_h(p, r, resp.is_pointer_button_down_on() || on, false);
            let blink = !on || (ui.input(|i| i.time) * 2.0) as i64 % 2 == 0;
            let dot = Pos2::new(r.min.x + 14.0, r.center().y);
            if blink {
                p.circle_filled(dot, 5.0, if on { red } else { pal().ink });
            } else {
                p.circle_stroke(dot, 5.0, Stroke::new(1.5_f32, red));
            }
            ptext(p, Pos2::new(r.min.x + 26.0, r.center().y), Align::Min, if on { "STOP" } else { "REC" }, 2.0, if on { red } else { pal().ink });
            if resp.clicked() {
                self.rec_toggle();
            }
            if on {
                ui.ctx().request_repaint_after(Duration::from_millis(250));
            }
            let secs = self.rec_t0.map(|t| t.elapsed().as_secs_f32()).unwrap_or(0.0);
            lcd_box(ui, &fmt_time(secs), 64.0, if on { red } else { pal().ink });
            // the level: how loud the last moment was, falling back down between peaks
            let peak = self
                .rec_buf
                .as_ref()
                .and_then(|b| {
                    b.lock().ok().map(|mut b| {
                        let pk = b.peak;
                        b.peak = pk * 0.85;
                        pk
                    })
                })
                .unwrap_or(0.0);
            let (r, _) = ui.allocate_exact_size(Vec2::new(110.0, bh()), Sense::hover());
            let bar = r.shrink2(Vec2::new(0.0, 6.0));
            ui.painter().rect_filled(bar, 2.0, pal().display);
            let w = bar.width() * peak.clamp(0.0, 1.0);
            let col = if peak > 0.9 {
                red
            } else if peak > 0.6 {
                Color32::from_rgb(217, 164, 65)
            } else {
                Color32::from_rgb(111, 191, 115)
            };
            ui.painter().rect_filled(Rect::from_min_size(bar.min, Vec2::new(w, bar.height())), 2.0, col);
            // (an edge round it, so the empty meter shows on the screen's glass)
            ui.painter().rect_stroke(bar, 0.0, Stroke::new(1.0_f32, pal().ink));
            label(ui, "LEVEL", 0.0);
        });
        if self.rec_waiting {
            dim_centered(ui, "Allow the microphone to record yourself");
        }
        if let Some(i) = self.rec_ask {
            centered_row(ui, "rec_ask", |ui| {
                label(ui, "KEEP THE LAST TAKE?", 0.0);
                if retro_btn(ui, "KEEP", false).clicked() {
                    self.rec_ask = None;
                }
                if retro_btn(ui, "DELETE", false).clicked() {
                    self.delete_take(i);
                }
            });
        }
        if let Some(e) = self.rec_buf.as_ref().and_then(|b| b.lock().ok().map(|b| b.err.clone())).filter(|e| !e.is_empty()) {
            dim_line(ui, &e, 1.0, red);
        }
        // ---- the sound coming in: the last eight seconds, scrolling
        let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 110.0), Sense::hover());
        let p = ui.painter();
        inset(p, r, pal().display);
        let mid = r.center().y;
        p.line_segment([Pos2::new(r.min.x, mid), Pos2::new(r.max.x, mid)], Stroke::new(1.0_f32, pal().dim.gamma_multiply(0.5)));
        if let Some(b) = self.rec_buf.as_ref().and_then(|b| b.lock().ok()) {
            let span = (b.rate as usize * 8).max(1);
            let n = b.samples.len();
            let start = n.saturating_sub(span);
            let cols = r.width().max(1.0) as usize;
            let per = span / cols.max(1);
            for c in 0..cols {
                let a = start + c * per;
                let z = (a + per).min(n);
                if a >= z {
                    continue;
                }
                let (mut lo, mut hi) = (0i16, 0i16);
                for s in &b.samples[a..z] {
                    lo = lo.min(*s);
                    hi = hi.max(*s);
                }
                let x = r.min.x + c as f32;
                let h = r.height() / 2.0 - 4.0;
                let y0 = mid - hi as f32 / 32768.0 * h;
                let y1 = mid - lo as f32 / 32768.0 * h;
                p.line_segment([Pos2::new(x, y0), Pos2::new(x, y1.max(y0 + 1.0))], Stroke::new(1.0_f32, pal().bar_txt));
            }
            ui.ctx().request_repaint();
        } else {
            ptext(p, r.center(), Align::Center, "PRESS REC AND PLAY", 2.0, pal().dim);
        }
        // ---- this song's takes, newest first: tap one to hear it (again to stop)
        let vid = self.cur.as_ref().map(|c| c.vid.clone()).unwrap_or_default();
        let mine: Vec<usize> = (0..self.takes.len()).rev().filter(|i| self.takes[*i].vid == vid).collect();
        if mine.is_empty() {
            return;
        }
        ui.add_space(6.0);
        crate::songs::heading(ui, "TAKES", ui.available_width());
        let playing = self.take_playing.map(|(k, _)| k);
        let mut tap = None;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
            for &i in &mine {
                let t = &self.takes[i];
                let name = format!("{}   {}", t.stamp, fmt_time(t.secs));
                if retro_btn(ui, &name, playing == Some(i)).clicked() {
                    tap = Some(i);
                }
            }
        });
        if let Some(i) = tap {
            self.take_del = Some(i);
            self.play_take(i);
        }
        if let Some(i) = self.take_del.filter(|i| mine.contains(i)) {
            ui.horizontal_wrapped(|ui| {
                let t = &self.takes[i];
                label(ui, &format!("{}  {}", t.stamp, if playing == Some(i) { "PLAYING" } else { "" }), 0.0);
                if playing == Some(i) && retro_btn(ui, "STOP", false).clicked() {
                    self.player.send(Cmd::StopOnce);
                    self.take_playing = None;
                }
                if retro_btn(ui, "DELETE", false).clicked() {
                    self.delete_take(i);
                }
            });
        }
    }
}
