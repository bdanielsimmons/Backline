//! Recording yourself: your guitar (or the mic) through the audio interface, while Backline plays the band, a loop
//! or a song. The input is kept as it comes in (for the live waveform), and when you stop, two files are written:
//! you alone, and you over what Backline played (the player keeps what it plays, as it does for the tour demos).

use rodio::cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// What has come in so far: mono, at the device's own rate; when the first samples arrived (so they line up with
/// what Backline played); and the loudest moment lately (the level meter).
pub struct InBuf {
    pub rate: u32,
    pub samples: Vec<i16>,
    pub started: Option<Instant>,
    pub peak: f32,
    pub err: String,
}

/// Which take is on (the input thread stops when it is no longer the current one), and that thread.
static GEN: AtomicU32 = AtomicU32::new(0);
static THREAD: Mutex<Option<std::thread::JoinHandle<()>>> = Mutex::new(None);

fn lock(b: &Arc<Mutex<InBuf>>) -> std::sync::MutexGuard<'_, InBuf> {
    b.lock().unwrap_or_else(|e| e.into_inner())
}

/// The inputs this computer has (the interface, the mic), the default first.
pub fn input_devices() -> Vec<String> {
    let host = rodio::cpal::default_host();
    let def = host.default_input_device().and_then(|d| d.name().ok());
    let mut out: Vec<String> = Vec::new();
    if let Some(d) = def.clone() {
        out.push(d);
    }
    if let Ok(devs) = host.input_devices() {
        for d in devs {
            if let Ok(n) = d.name() {
                if !out.contains(&n) {
                    out.push(n);
                }
            }
        }
    }
    out
}

/// Start taking sound in from this input (None: the default one). The stream lives on its own thread until
/// `stop`; what comes in lands in the buffer returned.
pub fn start(device: Option<String>) -> Arc<Mutex<InBuf>> {
    let buf = Arc::new(Mutex::new(InBuf { rate: 44100, samples: Vec::new(), started: None, peak: 0.0, err: String::new() }));
    // the take before this one lets go of the input first
    let gen = GEN.fetch_add(1, Ordering::SeqCst) + 1;
    if let Some(h) = THREAD.lock().unwrap_or_else(|e| e.into_inner()).take() {
        let _ = h.join();
    }
    let b = buf.clone();
    let handle = std::thread::spawn(move || {
        let host = rodio::cpal::default_host();
        let dev = match device {
            Some(name) => host.input_devices().ok().and_then(|mut it| it.find(|d| d.name().ok().as_deref() == Some(&name))),
            None => None,
        }
        .or_else(|| host.default_input_device());
        let Some(dev) = dev else {
            lock(&b).err = "NO INPUT FOUND - PLUG IN THE INTERFACE OR A MIC".to_string();
            return;
        };
        let cfg = match dev.default_input_config() {
            Ok(c) => c,
            Err(e) => {
                lock(&b).err = format!("INPUT: {}", e);
                return;
            }
        };
        let ch = cfg.channels().max(1) as usize;
        lock(&b).rate = cfg.sample_rate().0;
        let bb = b.clone();
        let err_b = b.clone();
        let on_err = move |e: rodio::cpal::StreamError| {
            lock(&err_b).err = format!("INPUT: {}", e);
        };
        // any sample format: everything becomes mono i16
        let stream = match cfg.sample_format() {
            rodio::cpal::SampleFormat::F32 => dev.build_input_stream(
                &cfg.into(),
                move |d: &[f32], _: &_| push(&bb, d.iter().map(|x| (x * 32767.0) as i32), ch),
                on_err,
                None,
            ),
            rodio::cpal::SampleFormat::I16 => dev.build_input_stream(
                &cfg.into(),
                move |d: &[i16], _: &_| push(&bb, d.iter().map(|x| *x as i32), ch),
                on_err,
                None,
            ),
            rodio::cpal::SampleFormat::U16 => dev.build_input_stream(
                &cfg.into(),
                move |d: &[u16], _: &_| push(&bb, d.iter().map(|x| *x as i32 - 32768), ch),
                on_err,
                None,
            ),
            f => {
                lock(&b).err = format!("INPUT: {} SAMPLES NOT SUPPORTED", f);
                return;
            }
        };
        let stream = match stream {
            Ok(s) => s,
            Err(e) => {
                lock(&b).err = format!("INPUT: {}", e);
                return;
            }
        };
        if let Err(e) = stream.play() {
            lock(&b).err = format!("INPUT: {}", e);
            return;
        }
        while GEN.load(Ordering::SeqCst) == gen {
            std::thread::sleep(std::time::Duration::from_millis(40));
        }
        drop(stream);
    });
    *THREAD.lock().unwrap_or_else(|e| e.into_inner()) = Some(handle);
    buf
}

/// Frames in (any channel count, as i32 in i16's range) to the buffer, as mono.
fn push(b: &Arc<Mutex<InBuf>>, frames: impl Iterator<Item = i32>, ch: usize) {
    let mut mono: Vec<i16> = Vec::new();
    let mut acc = 0i32;
    let mut k = 0;
    let mut peak = 0i32;
    for s in frames {
        acc += s;
        k += 1;
        if k == ch {
            let m = (acc / ch as i32).clamp(-32768, 32767);
            peak = peak.max(m.abs());
            mono.push(m as i16);
            acc = 0;
            k = 0;
        }
    }
    let n = mono.len();
    {
        let mut buf = lock(b);
        if buf.started.is_none() {
            // this batch started as long ago as it lasts
            buf.started = Some(Instant::now() - std::time::Duration::from_secs_f64(n as f64 / buf.rate.max(1) as f64));
        }
        buf.samples.extend(mono);
        buf.peak = buf.peak.max(peak as f32 / 32768.0);
    }
}

/// The take ends: the input thread lets go of the device.
pub fn stop() {
    GEN.fetch_add(1, Ordering::SeqCst);
    if let Some(h) = THREAD.lock().unwrap_or_else(|e| e.into_inner()).take() {
        let _ = h.join();
    }
}

/// Mono samples at `from` Hz as mono at `to` Hz (straight lines between samples).
pub fn resample(s: &[i16], from: u32, to: u32) -> Vec<i16> {
    if from == to || s.is_empty() {
        return s.to_vec();
    }
    let n = (s.len() as f64 * to as f64 / from as f64) as usize;
    let step = from as f64 / to as f64;
    (0..n)
        .map(|i| {
            let x = i as f64 * step;
            let j = x as usize;
            let f = (x - j as f64) as f32;
            let a = s[j.min(s.len() - 1)] as f32;
            let b = s[(j + 1).min(s.len() - 1)] as f32;
            (a + (b - a) * f) as i16
        })
        .collect()
}

/// You (mono, 44.1 kHz) over what Backline played (stereo, 44.1 kHz), you a little louder: stereo.
pub fn over(you: &[i16], back: &[i16], offset_frames: i64) -> Vec<i16> {
    let frames = (you.len() as i64 + offset_frames.max(0)).max(back.len() as i64 / 2) as usize;
    (0..frames * 2)
        .map(|i| {
            let fr = (i / 2) as i64 - offset_frames;
            let y = if fr >= 0 && (fr as usize) < you.len() { you[fr as usize] as f32 } else { 0.0 };
            let b = back.get(i).copied().unwrap_or(0) as f32 * 0.6;
            (y + b).clamp(-32768.0, 32767.0) as i16
        })
        .collect()
}
