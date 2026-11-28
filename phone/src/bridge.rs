//! Backline as one of the phone's players (the Java side: phone/java/.../Bridge.java and Playback.java): audio focus
//! (other players pause when Backline plays; Backline pauses when they play, and for a call, coming back after it),
//! the media session (the lock screen, the notification's controls, headphone buttons) and the service that keeps the
//! song going in the background. What the phone asks for is done to the player straight away where it can be (pause,
//! play, seek: in the background eframe draws nothing and the app's frame doesn't run), and passed to the app's next
//! frame (`take`), which does the rest (Next, Previous, starting a song).

use crate::player::Cmd;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Mutex, OnceLock};

/// What happened, for the app's next frame.
pub(crate) enum Ev {
    /// paused, already in the player: by another player or a call (`focus`), or from the lock screen
    Paused { focus: bool },
    /// playing again, already
    Resumed,
    /// play asked for with nothing playing (the app starts the song)
    Play,
    Next,
    Prev,
    Stop,
    Seek(f32),
}

/// The app's playing as this side knows it: 0 stopped, 1 playing, 2 paused, 3 paused for a call (plays again after).
static STATE: AtomicU8 = AtomicU8::new(0);
static EVENTS: Mutex<Vec<Ev>> = Mutex::new(Vec::new());

struct Link {
    tx: Mutex<Sender<Cmd>>,
    ctx: eframe::egui::Context,
}
static LINK: OnceLock<Link> = OnceLock::new();

#[cfg(target_os = "android")]
static CLASS: OnceLock<jni::objects::GlobalRef> = OnceLock::new();

/// What the phone asked for since the last frame.
pub(crate) fn take() -> Vec<Ev> {
    EVENTS.lock().map(|mut e| std::mem::take(&mut *e)).unwrap_or_default()
}

/// The app's playing, each frame (a pause for a call stays one until the call's over or you play).
pub(crate) fn set_state(s: u8) {
    if !(STATE.load(Ordering::Relaxed) == 3 && s == 2) {
        STATE.store(s, Ordering::Relaxed);
    }
}

/// What the phone asked for (from Java): done to the player where it can be, passed on to the app. Returns whether
/// the song now plays, when that changed (the session shows it).
fn handle(what: i32, arg: i64) -> Option<bool> {
    let link = LINK.get()?;
    let send = |c: Cmd| {
        if let Ok(tx) = link.tx.lock() {
            let _ = tx.send(c);
        }
    };
    let st = STATE.load(Ordering::Relaxed);
    let mut q = EVENTS.lock().ok()?;
    let shown = match what {
        // audio focus: lost to another player (-1) or for a while, a call or the assistant (-2); back (1)
        1 => match arg {
            -1 | -2 if st == 1 => {
                send(Cmd::Pause);
                STATE.store(if arg == -2 { 3 } else { 2 }, Ordering::Relaxed);
                q.push(Ev::Paused { focus: true });
                Some(false)
            }
            1 if st == 3 => {
                send(Cmd::Resume);
                STATE.store(1, Ordering::Relaxed);
                q.push(Ev::Resumed);
                Some(true)
            }
            _ => None,
        },
        // play, pause (the lock screen, the notification, headphones)
        2 => match st {
            2 | 3 => {
                send(Cmd::Resume);
                STATE.store(1, Ordering::Relaxed);
                q.push(Ev::Resumed);
                Some(true)
            }
            0 => {
                q.push(Ev::Play);
                None
            }
            _ => None,
        },
        3 if st == 1 => {
            send(Cmd::Pause);
            STATE.store(2, Ordering::Relaxed);
            q.push(Ev::Paused { focus: false });
            Some(false)
        }
        4 => {
            q.push(Ev::Next);
            None
        }
        5 => {
            q.push(Ev::Prev);
            None
        }
        6 => {
            if st == 1 {
                send(Cmd::Pause);
                STATE.store(2, Ordering::Relaxed);
            }
            q.push(Ev::Stop);
            Some(false)
        }
        // a seek (the lock screen's bar): straight to the player when there's a song in it
        7 => {
            let t = arg.max(0) as f32 / 1000.0;
            if st == 0 {
                q.push(Ev::Seek(t));
            } else {
                send(Cmd::Seek(t));
            }
            None
        }
        _ => None,
    };
    drop(q);
    link.ctx.request_repaint();
    shown
}

#[cfg(target_os = "android")]
extern "system" fn event(mut env: jni::JNIEnv, cls: jni::objects::JClass, what: jni::sys::jint, arg: jni::sys::jlong) {
    if let Some(play) = handle(what, arg) {
        let _ = env.call_static_method(&cls, "state", "(Z)V", &[jni::objects::JValue::Bool(play as u8)]);
        if env.exception_check().unwrap_or(false) {
            let _ = env.exception_clear();
        }
    }
}

#[cfg(target_os = "android")]
fn with_env<R>(f: impl FnOnce(&mut jni::JNIEnv, &jni::objects::JObject) -> Option<R>) -> Option<R> {
    let ctx = ndk_context::android_context();
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }.ok()?;
    let mut env = vm.attach_current_thread().ok()?;
    let app = unsafe { jni::objects::JObject::from_raw(ctx.context().cast()) };
    let r = f(&mut env, &app);
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_describe();
        let _ = env.exception_clear();
    }
    r
}

/// At start: the Java side found (through the app's own class loader) and told where to send what the phone asks for.
pub(crate) fn init(tx: Sender<Cmd>, ctx: eframe::egui::Context) {
    let _ = LINK.set(Link { tx: Mutex::new(tx), ctx });
    #[cfg(target_os = "android")]
    {
        use jni::objects::JClass;
        let r = with_env(|env, app| {
            let loader = env.call_method(app, "getClassLoader", "()Ljava/lang/ClassLoader;", &[]).ok()?.l().ok()?;
            let name = env.new_string("com.v9studio.backline.practice.Bridge").ok()?;
            let cls = env.call_method(&loader, "loadClass", "(Ljava/lang/String;)Ljava/lang/Class;", &[(&name).into()]).ok()?.l().ok()?;
            let cls = JClass::from(cls);
            let m = jni::NativeMethod { name: "event".into(), sig: "(IJ)V".into(), fn_ptr: event as *mut std::ffi::c_void };
            env.register_native_methods(&cls, &[m]).ok()?;
            env.new_global_ref(&cls).ok()
        });
        match r {
            Some(g) => {
                let _ = CLASS.set(g);
            }
            None => crate::api::log("the phone's media controls couldn't be set up"),
        }
    }
}

/// Ask for the phone's audio (other players pause), or give it back.
pub(crate) fn focus(on: bool) {
    #[cfg(target_os = "android")]
    {
        let _ = with_env(|env, app| {
            let cls: &jni::objects::JClass = CLASS.get()?.as_obj().into();
            if on {
                env.call_static_method(cls, "requestFocus", "(Landroid/content/Context;)Z", &[app.into()]).ok()?;
            } else {
                env.call_static_method(cls, "abandonFocus", "()V", &[]).ok()?;
            }
            Some(())
        });
    }
    #[cfg(not(target_os = "android"))]
    let _ = on;
}

/// What's playing, for the phone's media controls.
pub(crate) struct Now<'a> {
    pub title: &'a str,
    pub artist: &'a str,
    pub dur: f32,
    pub pos: f32,
    pub playing: bool,
    pub speed: f32,
    /// a new cover (PNG; empty: none), or the one shown kept
    pub art: Option<&'a [u8]>,
}

/// The lock screen, the notification and their controls: what's playing, or (none) nothing.
pub(crate) fn session(now: Option<Now>) {
    #[cfg(target_os = "android")]
    {
        use jni::objects::{JObject, JValue};
        let _ = with_env(|env, app| {
            let cls: &jni::objects::JClass = CLASS.get()?.as_obj().into();
            let sig = "(Landroid/content/Context;Ljava/lang/String;Ljava/lang/String;JJZF[BZ)V";
            match &now {
                Some(n) => {
                    let t = env.new_string(n.title).ok()?;
                    let a = env.new_string(n.artist).ok()?;
                    let art = match n.art {
                        Some(b) => JObject::from(env.byte_array_from_slice(b).ok()?),
                        None => JObject::null(),
                    };
                    env.call_static_method(
                        cls,
                        "update",
                        sig,
                        &[
                            app.into(),
                            (&t).into(),
                            (&a).into(),
                            JValue::Long((n.dur * 1000.0) as i64),
                            JValue::Long((n.pos * 1000.0) as i64),
                            JValue::Bool(n.playing as u8),
                            JValue::Float(n.speed),
                            (&art).into(),
                            JValue::Bool(1),
                        ],
                    )
                    .ok()?;
                }
                None => {
                    let e = env.new_string("").ok()?;
                    env.call_static_method(
                        cls,
                        "update",
                        sig,
                        &[app.into(), (&e).into(), (&e).into(), JValue::Long(0), JValue::Long(0), JValue::Bool(0), JValue::Float(1.0), JValue::Object(&JObject::null()), JValue::Bool(0)],
                    )
                    .ok()?;
                }
            }
            Some(())
        });
    }
    #[cfg(not(target_os = "android"))]
    let _ = now;
}
