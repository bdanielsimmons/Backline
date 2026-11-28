//! Backline on an iPhone: what the phone itself is asked for, through Objective-C. The sound session (Backline plays
//! alongside other apps until you press play, then they stop; a call or another app pauses it, and after a call it
//! carries on), the lock screen and Control Center (the song, its cover, play/pause, Previous, Next, the position),
//! the screen's edges (the notch or island on top, the home bar along the bottom), the screen kept on, the clipboard,
//! the mic. What the phone asks for goes through `bridge` like Android's does, so the app treats both alike.

use crate::bridge;
use block2::RcBlock;
use objc2::encode::{Encode, Encoding, RefEncode};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Bool};
use objc2::{class, msg_send, msg_send_id};
use objc2_foundation::NSString;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
struct Size {
    w: f64,
    h: f64,
}
unsafe impl Encode for Size {
    const ENCODING: Encoding = Encoding::Struct("CGSize", &[f64::ENCODING, f64::ENCODING]);
}
unsafe impl RefEncode for Size {
    const ENCODING_REF: Encoding = Encoding::Pointer(&Self::ENCODING);
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}
unsafe impl Encode for Rect {
    const ENCODING: Encoding =
        Encoding::Struct("CGRect", &[Encoding::Struct("CGPoint", &[f64::ENCODING, f64::ENCODING]), Encoding::Struct("CGSize", &[f64::ENCODING, f64::ENCODING])]);
}
unsafe impl RefEncode for Rect {
    const ENCODING_REF: Encoding = Encoding::Pointer(&Self::ENCODING);
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
struct Insets {
    top: f64,
    left: f64,
    bottom: f64,
    right: f64,
}
unsafe impl Encode for Insets {
    const ENCODING: Encoding = Encoding::Struct("UIEdgeInsets", &[f64::ENCODING, f64::ENCODING, f64::ENCODING, f64::ENCODING]);
}
unsafe impl RefEncode for Insets {
    const ENCODING_REF: Encoding = Encoding::Pointer(&Self::ENCODING);
}

#[link(name = "AVFoundation", kind = "framework")]
extern "C" {
    static AVAudioSessionCategoryPlayAndRecord: &'static NSString;
    static AVAudioSessionModeDefault: &'static NSString;
    static AVAudioSessionInterruptionNotification: &'static NSString;
    static AVAudioSessionInterruptionTypeKey: &'static NSString;
    static AVAudioSessionInterruptionOptionKey: &'static NSString;
    static AVAudioSessionRouteChangeNotification: &'static NSString;
    static AVAudioSessionRouteChangeReasonKey: &'static NSString;
    static AVAudioSessionMediaServicesWereResetNotification: &'static NSString;
}

#[link(name = "MediaPlayer", kind = "framework")]
extern "C" {
    static MPMediaItemPropertyTitle: &'static NSString;
    static MPMediaItemPropertyArtist: &'static NSString;
    static MPMediaItemPropertyPlaybackDuration: &'static NSString;
    static MPMediaItemPropertyArtwork: &'static NSString;
    static MPNowPlayingInfoPropertyElapsedPlaybackTime: &'static NSString;
    static MPNowPlayingInfoPropertyPlaybackRate: &'static NSString;
}

// the sound session's options: on the speaker (not the earpiece, as recording would have it), Bluetooth headphones
// in full quality, and (until you play) alongside other apps' sound
const SPEAKER: usize = 0x8;
const A2DP: usize = 0x20;
const MIX: usize = 0x1;

/// The app's folder the Files app shows (On My iPhone > Backline): your songs go here.
pub(crate) fn documents() -> std::path::PathBuf {
    std::path::PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join("Documents")
}

/// At start, before anything plays: where things are kept, crashes written down, and the sound session set up to play
/// alongside other apps (the player's output opens at start; that mustn't stop someone's music).
pub(crate) fn setup() {
    let home = std::path::PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
    let own = home.join("Library").join("Application Support").join("Backline");
    let _ = std::fs::create_dir_all(&own);
    let _ = std::fs::create_dir_all(documents());
    std::env::set_var("BACKLINE_HOME", &own);
    // (a test copy: a file named MUTE beside the settings plays silently)
    if own.join("MUTE").exists() {
        crate::player::MUTED.store(true, Ordering::Relaxed);
    }
    std::panic::set_hook(Box::new(|p| crate::api::log(&format!("panic: {}", p))));
    session_options(MIX);
}

fn audio_session() -> Option<Retained<AnyObject>> {
    unsafe { msg_send_id![class!(AVAudioSession), sharedInstance] }
}

fn session_options(opts: usize) {
    let Some(s) = audio_session() else { return };
    let ok: bool = unsafe {
        msg_send![&s, setCategory: AVAudioSessionCategoryPlayAndRecord, mode: AVAudioSessionModeDefault, options: SPEAKER | A2DP | opts, error: std::ptr::null_mut::<*mut AnyObject>()]
    };
    if !ok {
        crate::api::log("the sound session couldn't be set up");
    }
}

/// Playing (other apps' sound stops, and Backline is what the lock screen shows), or done (others may play again).
pub(crate) fn focus(on: bool) {
    let Some(s) = audio_session() else { return };
    if on {
        session_options(0);
        let ok: bool = unsafe { msg_send![&s, setActive: true, withOptions: 0usize, error: std::ptr::null_mut::<*mut AnyObject>()] };
        if !ok {
            crate::api::log("the sound session couldn't start");
        }
    } else {
        session_options(MIX);
        // (refused while the output runs, which is fine: it just stays on, mixing)
        let _: bool = unsafe { msg_send![&s, setActive: false, withOptions: 1usize, error: std::ptr::null_mut::<*mut AnyObject>()] };
    }
}

/// The player's output, opened again: the system stops an app's sound for a call or another app, and it doesn't come
/// back by itself.
fn revive() {
    if let Some(s) = audio_session() {
        let _: bool = unsafe { msg_send![&s, setActive: true, withOptions: 0usize, error: std::ptr::null_mut::<*mut AnyObject>()] };
    }
    bridge::send_player(crate::player::Cmd::Revive);
}

static WATCHING: AtomicBool = AtomicBool::new(false);
/// (an interruption under way: the output needs opening again once it's over)
static CUT: AtomicBool = AtomicBool::new(false);

/// At start (on the main thread): the lock screen's buttons, and the phone's word on calls, other apps and
/// headphones coming out, all passed to `bridge` as Android's are.
pub(crate) fn watch() {
    if WATCHING.swap(true, Ordering::Relaxed) {
        return;
    }
    unsafe {
        let Some(cc): Option<Retained<AnyObject>> = msg_send_id![class!(MPRemoteCommandCenter), sharedCommandCenter] else { return };
        let add = |name: &str, what: i32| {
            let cmd: Option<Retained<AnyObject>> = match name {
                "play" => msg_send_id![&cc, playCommand],
                "pause" => msg_send_id![&cc, pauseCommand],
                "toggle" => msg_send_id![&cc, togglePlayPauseCommand],
                "next" => msg_send_id![&cc, nextTrackCommand],
                "prev" => msg_send_id![&cc, previousTrackCommand],
                "stop" => msg_send_id![&cc, stopCommand],
                _ => msg_send_id![&cc, changePlaybackPositionCommand],
            };
            let Some(cmd) = cmd else { return };
            let block = RcBlock::new(move |ev: *mut AnyObject| -> isize {
                match what {
                    // (play/pause from headphones: whichever it isn't now)
                    0 => {
                        let w = if bridge::playing() { 3 } else { 2 };
                        command(w, 0);
                    }
                    7 => {
                        let t: f64 = if ev.is_null() { 0.0 } else { msg_send![ev, positionTime] };
                        command(7, (t * 1000.0) as i64);
                    }
                    w => command(w, 0),
                }
                0
            });
            let _: () = msg_send![&cmd, setEnabled: true];
            let _: Option<Retained<AnyObject>> = msg_send_id![&cmd, addTargetWithHandler: &*block];
        };
        add("play", 2);
        add("pause", 3);
        add("toggle", 0);
        add("next", 4);
        add("prev", 5);
        add("stop", 6);
        add("seek", 7);

        let Some(nc): Option<Retained<AnyObject>> = msg_send_id![class!(NSNotificationCenter), defaultCenter] else { return };
        let Some(main): Option<Retained<AnyObject>> = msg_send_id![class!(NSOperationQueue), mainQueue] else { return };
        let observe = |name: &NSString, f: fn(*mut AnyObject)| {
            let block = RcBlock::new(move |note: *mut AnyObject| f(note));
            let token: Option<Retained<AnyObject>> =
                msg_send_id![&nc, addObserverForName: name, object: std::ptr::null_mut::<AnyObject>(), queue: &*main, usingBlock: &*block];
            // (kept for good: the app watches for as long as it runs)
            std::mem::forget(token);
        };
        observe(AVAudioSessionInterruptionNotification, interruption);
        observe(AVAudioSessionRouteChangeNotification, route_change);
        observe(AVAudioSessionMediaServicesWereResetNotification, |_| revive());
    }
}

fn number_for(note: *mut AnyObject, key: &NSString) -> Option<usize> {
    unsafe {
        let info: Option<Retained<AnyObject>> = msg_send_id![note, userInfo];
        let n: Option<Retained<AnyObject>> = msg_send_id![&info?, objectForKey: key];
        Some(msg_send![&n?, unsignedIntegerValue])
    }
}

/// A call, an alarm, Siri or another app's music: Backline pauses (and after a call, carries on).
fn interruption(note: *mut AnyObject) {
    let kind = unsafe { number_for(note, AVAudioSessionInterruptionTypeKey) }.unwrap_or(0);
    if kind == 1 {
        CUT.store(true, Ordering::Relaxed);
        // (paused "for a while": it plays again if the phone says to when it's over)
        command(1, -2);
    } else {
        let resume = unsafe { number_for(note, AVAudioSessionInterruptionOptionKey) }.unwrap_or(0) & 1 == 1;
        if CUT.swap(false, Ordering::Relaxed) {
            revive();
        }
        if resume {
            command(1, 1);
        }
    }
}

/// Headphones coming out: Backline pauses, as music apps do, rather than playing out loud.
fn route_change(note: *mut AnyObject) {
    let why = unsafe { number_for(note, AVAudioSessionRouteChangeReasonKey) }.unwrap_or(0);
    if why == 2 {
        command(3, 0);
    }
}

/// What the phone asked for, to `bridge` (as Android's Java side does), and the lock screen kept in step.
fn command(what: i32, arg: i64) {
    if let Some(play) = bridge::from_phone(what, arg) {
        playing_now(play);
    }
}

// what the lock screen shows: where the song was at a moment, and how fast it goes
struct Clock {
    pos: f64,
    at: Option<Instant>,
    rate: f64,
}
static CLOCK: Mutex<Clock> = Mutex::new(Clock { pos: 0.0, at: None, rate: 1.0 });
/// (the cover sent last, by a number for its bytes, so it isn't remade every time)
static ART_KEY: AtomicIsize = AtomicIsize::new(0);
thread_local! {
    static ART: std::cell::RefCell<Option<Retained<AnyObject>>> = const { std::cell::RefCell::new(None) };
}

fn center() -> Option<Retained<AnyObject>> {
    unsafe { msg_send_id![class!(MPNowPlayingInfoCenter), defaultCenter] }
}

fn number(v: f64) -> Retained<AnyObject> {
    unsafe { msg_send_id![class!(NSNumber), numberWithDouble: v] }
}

/// The song on the lock screen and in Control Center (none: nothing there). On the main thread.
pub(crate) fn now_playing(now: Option<&bridge::Now>) {
    let Some(c) = center() else { return };
    let Some(n) = now else {
        unsafe {
            let _: () = msg_send![&c, setNowPlayingInfo: std::ptr::null_mut::<AnyObject>()];
        }
        ART.with(|a| *a.borrow_mut() = None);
        ART_KEY.store(0, Ordering::Relaxed);
        return;
    };
    if let Ok(mut k) = CLOCK.lock() {
        k.pos = n.pos as f64;
        k.at = n.playing.then(Instant::now);
        k.rate = n.speed as f64;
    }
    unsafe {
        let d: Retained<AnyObject> = msg_send_id![class!(NSMutableDictionary), new];
        let put = |k: &NSString, v: &AnyObject| {
            let _: () = msg_send![&d, setObject: v, forKey: k];
        };
        put(MPMediaItemPropertyTitle, &NSString::from_str(n.title));
        put(MPMediaItemPropertyArtist, &NSString::from_str(n.artist));
        put(MPMediaItemPropertyPlaybackDuration, &number(n.dur as f64));
        put(MPNowPlayingInfoPropertyElapsedPlaybackTime, &number(n.pos as f64));
        put(MPNowPlayingInfoPropertyPlaybackRate, &number(if n.playing { n.speed as f64 } else { 0.0 }));
        if let Some(png) = n.art {
            let key = if png.is_empty() { 0 } else { png.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ *b as u64).wrapping_mul(0x100_0000_01b3)) as isize | 1 };
            if key != ART_KEY.swap(key, Ordering::Relaxed) {
                ART.with(|a| *a.borrow_mut() = if png.is_empty() { None } else { artwork(png) });
            }
        }
        ART.with(|a| {
            if let Some(art) = a.borrow().as_ref() {
                put(MPMediaItemPropertyArtwork, art);
            }
        });
        let _: () = msg_send![&c, setNowPlayingInfo: &*d];
    }
}

/// A cover for the lock screen, from the song's picture (PNG).
fn artwork(png: &[u8]) -> Option<Retained<AnyObject>> {
    unsafe {
        let data: Retained<AnyObject> = msg_send_id![class!(NSData), dataWithBytes: png.as_ptr() as *const std::ffi::c_void, length: png.len()];
        let img: Option<Retained<AnyObject>> = msg_send_id![class!(UIImage), imageWithData: &*data];
        let img = img?;
        let size: Size = msg_send![&img, size];
        let keep = img.clone();
        let block = RcBlock::new(move |_s: Size| -> *mut AnyObject { Retained::as_ptr(&keep) as *mut AnyObject });
        let a = msg_send_id![class!(MPMediaItemArtwork), alloc];
        let art: Option<Retained<AnyObject>> = msg_send_id![a, initWithBoundsSize: size, requestHandler: &*block];
        art
    }
}

/// Playing or paused, from the lock screen or another app (the position worked out from the last word).
fn playing_now(play: bool) {
    let Some(c) = center() else { return };
    let pos = match CLOCK.lock() {
        Ok(mut k) => {
            let p = k.pos + k.at.map(|t| t.elapsed().as_secs_f64() * k.rate).unwrap_or(0.0);
            k.pos = p;
            k.at = play.then(Instant::now);
            (p, k.rate)
        }
        Err(_) => (0.0, 1.0),
    };
    unsafe {
        let info: Option<Retained<AnyObject>> = msg_send_id![&c, nowPlayingInfo];
        let Some(info) = info else { return };
        let d: Retained<AnyObject> = msg_send_id![&info, mutableCopy];
        let _: () = msg_send![&d, setObject: &*number(pos.0), forKey: MPNowPlayingInfoPropertyElapsedPlaybackTime];
        let _: () = msg_send![&d, setObject: &*number(if play { pos.1 } else { 0.0 }), forKey: MPNowPlayingInfoPropertyPlaybackRate];
        let _: () = msg_send![&c, setNowPlayingInfo: &*d];
    }
}

fn ui_app() -> Option<Retained<AnyObject>> {
    unsafe { msg_send_id![class!(UIApplication), sharedApplication] }
}

/// The screen's size in its points (portrait), and the room the notch or island and the home bar take: (width,
/// height, top, bottom). On the main thread.
pub(crate) fn screen() -> Option<(f32, f32, f32, f32)> {
    unsafe {
        let scr: Option<Retained<AnyObject>> = msg_send_id![class!(UIScreen), mainScreen];
        let b: Rect = msg_send![&scr?, bounds];
        let app = ui_app()?;
        let wins: Option<Retained<AnyObject>> = msg_send_id![&app, windows];
        let win: Option<Retained<AnyObject>> = msg_send_id![&wins?, firstObject];
        let ins: Insets = match win {
            Some(w) => msg_send![&w, safeAreaInsets],
            None => Insets::default(),
        };
        Some((b.w as f32, b.h as f32, ins.top as f32, ins.bottom as f32))
    }
}

/// The screen's scale (pixels to a point).
pub(crate) fn scale() -> f32 {
    unsafe {
        let scr: Option<Retained<AnyObject>> = msg_send_id![class!(UIScreen), mainScreen];
        match scr {
            Some(s) => {
                let v: f64 = msg_send![&s, scale];
                v as f32
            }
            None => 1.0,
        }
    }
}

/// The screen kept on (while a song plays). On the main thread.
pub(crate) fn keep_awake(on: bool) {
    if let Some(app) = ui_app() {
        unsafe {
            let _: () = msg_send![&app, setIdleTimerDisabled: on];
        }
    }
}

/// What's on the clipboard. `asked`: you pressed PASTE (the phone may ask you to allow it); else only a copied link is
/// looked at, once (so coming back to the app doesn't ask every time).
pub(crate) fn clipboard(asked: bool) -> Option<String> {
    static SEEN: AtomicIsize = AtomicIsize::new(-1);
    unsafe {
        let pb: Option<Retained<AnyObject>> = msg_send_id![class!(UIPasteboard), generalPasteboard];
        let pb = pb?;
        if !asked {
            let n: isize = msg_send![&pb, changeCount];
            let has_link: bool = msg_send![&pb, hasURLs];
            if SEEN.swap(n, Ordering::Relaxed) == n || !has_link {
                return None;
            }
        }
        let s: Option<Retained<NSString>> = msg_send_id![&pb, string];
        s.map(|s| s.to_string())
    }
}

/// Whether the app may use the mic.
pub(crate) fn mic_allowed() -> bool {
    let Some(s) = audio_session() else { return false };
    let p: usize = unsafe { msg_send![&s, recordPermission] };
    // ('grnt')
    p == 0x6772_6e74
}

/// The phone's question: may Backline use the mic?
pub(crate) fn ask_mic() {
    if let Some(s) = audio_session() {
        let block = RcBlock::new(|ok: Bool| {
            crate::api::log(if ok.as_bool() { "the mic is allowed" } else { "the mic isn't allowed" });
        });
        unsafe {
            let _: () = msg_send![&s, requestRecordPermission: &*block];
        }
    }
}

/// The phone's offset from UTC in seconds.
pub(crate) fn local_offset() -> i64 {
    unsafe {
        let tz: Option<Retained<AnyObject>> = msg_send_id![class!(NSTimeZone), localTimeZone];
        match tz {
            Some(t) => {
                let s: isize = msg_send![&t, secondsFromGMT];
                s as i64
            }
            None => 0,
        }
    }
}
