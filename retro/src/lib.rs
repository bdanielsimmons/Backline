//! retro.dll: the classic Windows Media Player visualizers, as Ryan Circelli's wmp-legacy-visualizers (MIT) ports
//! them, run in V8. Backline downloads it from its site the first time one is picked (backline/src/ui/retro.rs).
//!
//! The script (retro.js) is the engine and js/entry.ts bundled by js/bundle.mjs. Everything here is a plain C
//! interface, called from one thread: Backline's visuals thread.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Mutex, Once};
use std::time::Instant;

/// Bumped when the interface or the script changes: Backline wants the library of its own version.
const VERSION: u32 = 1;
const BUNDLE: &str = include_str!("../retro.js");
const PRELUDE: &str =
    "globalThis.performance = { now: __now }; globalThis.console = { log: __log, warn: __log, error: __log, info: __log };";
/// The biggest frame: the engine draws 1920x1080 at most.
const OUT_PX: usize = 1920 * 1080;
/// Samples in at once: a second of stereo at 96 kHz.
const IN_FLOATS: usize = 96_000 * 2;

/// The bridge's functions (entry.ts), by their place here.
const NAMES: [&str; 10] = ["buffers", "open", "preset", "size", "rate", "push", "render", "smooth", "picture", "presets"];
const OPEN: usize = 1;
const PRESET: usize = 2;
const SIZE: usize = 3;
const RATE: usize = 4;
const PUSH: usize = 5;
const RENDER: usize = 6;
const SMOOTH: usize = 7;
const PICTURE: usize = 8;
const PRESETS: usize = 9;

pub struct Retro {
    funcs: Vec<v8::Global<v8::Function>>,
    context: v8::Global<v8::Context>,
    out: v8::SharedRef<v8::BackingStore>,
    inp: v8::SharedRef<v8::BackingStore>,
    err: String,
    // last, so it goes after the handles above
    isolate: v8::OwnedIsolate,
}

enum Arg<'a> {
    N(f64),
    S(&'a str),
    Bytes(Vec<u8>),
}

/// Why the last retro_new failed.
static NEW_ERR: Mutex<String> = Mutex::new(String::new());

fn now_cb(_s: &mut v8::PinScope, _a: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue) {
    static T0: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    rv.set_double(T0.get_or_init(Instant::now).elapsed().as_secs_f64() * 1000.0);
}

fn log_cb(_s: &mut v8::PinScope, _a: v8::FunctionCallbackArguments, _rv: v8::ReturnValue) {}

/// What the script threw, in a try-catch scope.
macro_rules! exception {
    ($tc:expr) => {
        $tc.exception().map(|e| e.to_rust_string_lossy($tc)).unwrap_or_else(|| "script stopped".to_string())
    };
}

fn run(s: &mut v8::PinScope, src: &str) -> Result<(), String> {
    v8::tc_scope!(let tc, s);
    let code = v8::String::new(tc, src).ok_or("script too long")?;
    match v8::Script::compile(tc, code, None).and_then(|sc| sc.run(tc)) {
        Some(_) => Ok(()),
        None => Err(exception!(tc)),
    }
}

type Setup = (v8::Global<v8::Context>, Vec<v8::Global<v8::Function>>, v8::SharedRef<v8::BackingStore>, v8::SharedRef<v8::BackingStore>);

fn setup(isolate: &mut v8::OwnedIsolate) -> Result<Setup, String> {
    v8::scope!(let hs, isolate);
    let global = v8::ObjectTemplate::new(hs);
    global.set(v8::String::new(hs, "__now").unwrap().into(), v8::FunctionTemplate::new(hs, now_cb).into());
    global.set(v8::String::new(hs, "__log").unwrap().into(), v8::FunctionTemplate::new(hs, log_cb).into());
    let context = v8::Context::new(hs, v8::ContextOptions { global_template: Some(global), ..Default::default() });
    let cs = &mut v8::ContextScope::new(hs, context);
    run(cs, PRELUDE)?;
    run(cs, BUNDLE)?;
    let gl = context.global(cs);
    let key = v8::String::new(cs, "BL").unwrap();
    let bl: v8::Local<v8::Object> = gl.get(cs, key.into()).and_then(|v| v.try_into().ok()).ok_or("the script has no BL")?;
    let mut funcs = Vec::new();
    for n in NAMES {
        let k = v8::String::new(cs, n).unwrap();
        let f: v8::Local<v8::Function> =
            bl.get(cs, k.into()).and_then(|v| v.try_into().ok()).ok_or_else(|| format!("the script has no BL.{n}"))?;
        funcs.push(v8::Global::new(cs, f));
    }
    // the frame and sample buffers, Backline's to read and fill, the script's to draw into and read
    let out = v8::ArrayBuffer::new_backing_store_from_bytes(vec![0u8; OUT_PX * 4]).make_shared();
    let inp = v8::ArrayBuffer::new_backing_store_from_bytes(vec![0u8; IN_FLOATS * 4]).make_shared();
    {
        v8::tc_scope!(let tc, cs);
        let f = v8::Local::new(tc, &funcs[0]);
        let a: [v8::Local<v8::Value>; 2] =
            [v8::ArrayBuffer::with_backing_store(tc, &out).into(), v8::ArrayBuffer::with_backing_store(tc, &inp).into()];
        let recv = v8::undefined(tc).into();
        if f.call(tc, recv, &a).is_none() {
            return Err(exception!(tc));
        }
    }
    Ok((v8::Global::new(cs, context), funcs, out, inp))
}

impl Retro {
    fn new() -> Result<Box<Retro>, String> {
        static INIT: Once = Once::new();
        INIT.call_once(|| {
            v8::V8::initialize_platform(v8::new_default_platform(0, false).make_shared());
            v8::V8::initialize();
        });
        let mut isolate = v8::Isolate::new(v8::CreateParams::default());
        let (context, funcs, out, inp) = setup(&mut isolate)?;
        Ok(Box::new(Retro { funcs, context, out, inp, err: String::new(), isolate }))
    }

    /// The bridge's function `f` with `args`: its result as a number, or as text.
    fn call(&mut self, f: usize, args: Vec<Arg>, text: bool) -> Result<(f64, String), String> {
        let Retro { isolate, context, funcs, .. } = self;
        v8::scope!(let hs, isolate);
        let ctx = v8::Local::new(hs, &*context);
        let cs = &mut v8::ContextScope::new(hs, ctx);
        v8::tc_scope!(let tc, cs);
        let func = v8::Local::new(tc, &funcs[f]);
        let mut argv: Vec<v8::Local<v8::Value>> = Vec::with_capacity(args.len());
        for a in args {
            argv.push(match a {
                Arg::N(n) => v8::Number::new(tc, n).into(),
                Arg::S(s) => v8::String::new(tc, s).ok_or("text too long")?.into(),
                Arg::Bytes(b) => {
                    let store = v8::ArrayBuffer::new_backing_store_from_bytes(b).make_shared();
                    v8::ArrayBuffer::with_backing_store(tc, &store).into()
                }
            });
        }
        let recv = v8::undefined(tc).into();
        match func.call(tc, recv, &argv) {
            Some(v) if text => Ok((0.0, v.to_rust_string_lossy(tc))),
            // (a function that returns nothing reads as 0)
            Some(v) => Ok((v.number_value(tc).filter(|n| !n.is_nan()).unwrap_or(0.0), String::new())),
            None => Err(exception!(tc)),
        }
    }

    fn num(&mut self, f: usize, args: Vec<Arg>) -> f64 {
        match self.call(f, args, false) {
            Ok((n, _)) => n,
            Err(e) => {
                self.err = e;
                -1.0
            }
        }
    }

    /// Background work (WebAssembly compiles) and promise jobs: once between frames.
    fn pump(&mut self) {
        let Retro { isolate, context, .. } = self;
        v8::scope!(let hs, isolate);
        let ctx = v8::Local::new(hs, &*context);
        let cs = &mut v8::ContextScope::new(hs, ctx);
        while v8::Platform::pump_message_loop(&v8::V8::get_current_platform(), cs, false) {}
        cs.perform_microtask_checkpoint();
    }
}

/// The library behind a handle, unless it's null.
unsafe fn get<'a>(r: *mut Retro) -> Option<&'a mut Retro> {
    r.as_mut()
}

/// Runs `f` on the library, a panic counting as a failure (`fail`).
fn guarded<T>(r: *mut Retro, fail: T, f: impl FnOnce(&mut Retro) -> T) -> T {
    let Some(r) = (unsafe { get(r) }) else { return fail };
    match catch_unwind(AssertUnwindSafe(|| f(r))) {
        Ok(v) => v,
        Err(_) => fail,
    }
}

fn copy_out(text: &str, buf: *mut u8, cap: usize) -> usize {
    if !buf.is_null() && text.len() <= cap {
        unsafe { std::ptr::copy_nonoverlapping(text.as_ptr(), buf, text.len()) };
    }
    text.len()
}

#[no_mangle]
pub extern "C" fn retro_version() -> u32 {
    VERSION
}

/// The engine, ready (null when it couldn't start: retro_error with a null handle says why).
#[no_mangle]
pub extern "C" fn retro_new() -> *mut Retro {
    let got = catch_unwind(Retro::new).unwrap_or_else(|_| Err("the engine crashed starting".to_string()));
    match got {
        Ok(r) => Box::into_raw(r),
        Err(e) => {
            *NEW_ERR.lock().unwrap_or_else(|p| p.into_inner()) = e;
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn retro_free(r: *mut Retro) {
    if !r.is_null() {
        let _ = catch_unwind(AssertUnwindSafe(|| drop(unsafe { Box::from_raw(r) })));
    }
}

/// Every preset, one per line (family key, preset number, family, name; tab-separated), into `buf` when it fits.
/// Returns its length in bytes.
#[no_mangle]
pub extern "C" fn retro_presets(r: *mut Retro, buf: *mut u8, cap: usize) -> usize {
    guarded(r, 0, |r| match r.call(PRESETS, vec![], true) {
        Ok((_, s)) => copy_out(&s, buf, cap),
        Err(e) => {
            r.err = e;
            0
        }
    })
}

/// A visualizer of its own: family `kind` (its key, UTF-8), preset `preset`, for a view `w` x `h` pixels. 1 if it
/// started.
#[no_mangle]
pub extern "C" fn retro_open(r: *mut Retro, kind: *const u8, kind_len: usize, preset: i32, w: i32, h: i32) -> i32 {
    guarded(r, 0, |r| {
        let kind = unsafe { std::slice::from_raw_parts(kind, kind_len) };
        let Ok(kind) = std::str::from_utf8(kind) else { return 0 };
        (r.num(OPEN, vec![Arg::S(kind), Arg::N(preset as f64), Arg::N(w as f64), Arg::N(h as f64)]) >= 0.0) as i32
    })
}

/// Another preset of the same family.
#[no_mangle]
pub extern "C" fn retro_preset(r: *mut Retro, preset: i32) -> i32 {
    guarded(r, 0, |r| (r.num(PRESET, vec![Arg::N(preset as f64)]) >= 0.0) as i32)
}

/// The view's size in pixels.
#[no_mangle]
pub extern "C" fn retro_size(r: *mut Retro, w: i32, h: i32) -> i32 {
    guarded(r, 0, |r| (r.num(SIZE, vec![Arg::N(w as f64), Arg::N(h as f64)]) >= 0.0) as i32)
}

/// The sound's sample rate.
#[no_mangle]
pub extern "C" fn retro_rate(r: *mut Retro, rate: i32) -> i32 {
    guarded(r, 0, |r| (r.num(RATE, vec![Arg::N(rate as f64)]) >= 0.0) as i32)
}

/// `n` interleaved stereo samples (-1..1), the sound since the last push.
#[no_mangle]
pub extern "C" fn retro_push(r: *mut Retro, pcm: *const f32, n: usize) -> i32 {
    guarded(r, 0, |r| {
        let n = n.min(IN_FLOATS) & !1;
        if n == 0 || pcm.is_null() {
            return 1;
        }
        let dst = r.inp.data().map_or(std::ptr::null_mut(), |p| p.as_ptr() as *mut f32);
        if dst.is_null() {
            return 0;
        }
        unsafe { std::ptr::copy_nonoverlapping(pcm, dst, n) };
        (r.num(PUSH, vec![Arg::N(n as f64)]) >= 0.0) as i32
    })
}

/// One frame (`state`: 2 playing, 1 paused, 0 stopped). Returns its width | height << 16 (0: none); its pixels, 0x00RRGGBB
/// row by row, are at retro_pixels until the next call.
#[no_mangle]
pub extern "C" fn retro_render(r: *mut Retro, state: i32) -> u32 {
    guarded(r, 0, |r| {
        let v = r.num(RENDER, vec![Arg::N(state as f64)]);
        r.pump();
        if v > 0.0 {
            v as u32
        } else {
            0
        }
    })
}

#[no_mangle]
pub extern "C" fn retro_pixels(r: *mut Retro) -> *const u32 {
    guarded(r, std::ptr::null(), |r| r.out.data().map_or(std::ptr::null(), |p| p.as_ptr() as *const u32))
}

/// 1 when this preset's frame is stretched smoothly, 0 when its pixels are kept sharp.
#[no_mangle]
pub extern "C" fn retro_smooth(r: *mut Retro) -> i32 {
    guarded(r, 0, |r| (r.num(SMOOTH, vec![]) > 0.0) as i32)
}

/// Picture Viz's photo, the song's cover: `w` x `h` RGBA (0 x 0: none).
#[no_mangle]
pub extern "C" fn retro_picture(r: *mut Retro, rgba: *const u8, w: i32, h: i32) -> i32 {
    guarded(r, 0, |r| {
        let n = (w.max(0) as usize) * (h.max(0) as usize) * 4;
        let bytes = if n > 0 && !rgba.is_null() { unsafe { std::slice::from_raw_parts(rgba, n) }.to_vec() } else { Vec::new() };
        let (w, h) = if bytes.is_empty() { (0, 0) } else { (w, h) };
        (r.num(PICTURE, vec![Arg::Bytes(bytes), Arg::N(w as f64), Arg::N(h as f64)]) >= 0.0) as i32
    })
}

/// Why the last call failed (with a null handle: why retro_new did), into `buf` when it fits. Returns its length.
#[no_mangle]
pub extern "C" fn retro_error(r: *mut Retro, buf: *mut u8, cap: usize) -> usize {
    match unsafe { get(r) } {
        Some(r) => copy_out(&r.err, buf, cap),
        None => copy_out(&NEW_ERR.lock().unwrap_or_else(|p| p.into_inner()), buf, cap),
    }
}
