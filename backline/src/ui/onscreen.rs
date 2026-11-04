//! Can anyone see the window? Not when it is minimized, or (on Windows) when other windows cover all of it. Then the
//! visuals stop, and so do the redraws while a song plays: there is no one to draw them for.
// (covering is only worked out on Windows)
#![cfg_attr(not(windows), allow(dead_code))]

use eframe::egui;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

static SEEN: AtomicBool = AtomicBool::new(true);
/// when it was last worked out whether other windows cover this one, and what was found
static LOOKED: Mutex<(f64, bool)> = Mutex::new((f64::NEG_INFINITY, false));

/// Can the window be seen (as of this frame)?
pub(crate) fn seen() -> bool {
    SEEN.load(Ordering::Relaxed)
}

/// Once a frame, before anything is drawn: is the window minimized, or covered? (Covered is looked at twice a second
/// at most, and never while the window is the one in front.)
pub(crate) fn check(ctx: &egui::Context, frame: &eframe::Frame) {
    #[cfg(windows)]
    if MAIN.load(Ordering::Relaxed) == 0 {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        if let Ok(RawWindowHandle::Win32(h)) = frame.window_handle().map(|h| h.as_raw()) {
            MAIN.store(h.hwnd.get(), Ordering::Relaxed);
        }
    }
    let (minimized, focused, now) = ctx.input(|i| (i.viewport().minimized == Some(true), i.focused, i.time));
    let behind = !minimized && !focused && {
        let mut l = LOOKED.lock().unwrap_or_else(|e| e.into_inner());
        if now - l.0 >= 0.5 {
            *l = (now, covered(frame));
        }
        l.1
    };
    // (recording a demo films the window itself, seen or not)
    let seen = (!minimized && !behind) || crate::tour::demo_kind().is_some();
    // (said in a test copy's log; yours would fill with every switch between windows)
    if SEEN.swap(seen, Ordering::Relaxed) != seen && crate::hidden_test_copy() {
        crate::api::log(if seen { "window in sight again" } else { "window out of sight: visuals and redraws paused" });
    }
}

/// A rectangle on the screen: left, top, right, bottom (right and bottom just past it).
type Area = (i32, i32, i32, i32);

/// What is left of `open` once `b` is taken away from it.
fn cut(open: &[Area], b: Area) -> Vec<Area> {
    let mut out = Vec::new();
    for &a in open {
        if b.0 >= a.2 || b.2 <= a.0 || b.1 >= a.3 || b.3 <= a.1 {
            out.push(a);
            continue;
        }
        // above and below it, then the two sides between them
        let (t, bm) = (b.1.max(a.1), b.3.min(a.3));
        if a.1 < t {
            out.push((a.0, a.1, a.2, t));
        }
        if bm < a.3 {
            out.push((a.0, bm, a.2, a.3));
        }
        if a.0 < b.0 {
            out.push((a.0, t, b.0, bm));
        }
        if b.2 < a.2 {
            out.push((b.2, t, a.2, bm));
        }
    }
    out
}

/// Do these windows (the ones in front, nearest first) cover all of `mine`?
fn covered_by(mine: Area, above: impl IntoIterator<Item = Area>) -> bool {
    let mut open = vec![mine];
    for b in above {
        open = cut(&open, b);
        if open.is_empty() {
            return true;
        }
        // (cut into too many pieces to follow: taken as seen)
        if open.len() > 200 {
            return false;
        }
    }
    false
}

#[cfg(windows)]
fn covered(frame: &eframe::Frame) -> bool {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    match frame.window_handle().map(|h| h.as_raw()) {
        Ok(RawWindowHandle::Win32(h)) => covered_window(windows::Win32::Foundation::HWND(h.hwnd.get())),
        _ => false,
    }
}

/// The main window (Windows: its handle, 0 until the first frame), for the monitor it is on.
static MAIN: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

pub(crate) fn main_window() -> isize {
    MAIN.load(Ordering::Relaxed)
}

#[cfg(not(windows))]
fn covered(_frame: &eframe::Frame) -> bool {
    false
}

/// Is this window covered by the windows in front of it? Windows you can see through (click-through overlays,
/// see-through ones, ones hidden away on another desktop) don't count, so a doubt comes out as "seen".
#[cfg(windows)]
pub(crate) fn covered_window(me: windows::Win32::Foundation::HWND) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{GetWindow, GW_HWNDPREV};
    let Some(mine) = bounds(me) else {
        return false;
    };
    let mut above = Vec::new();
    let mut w = unsafe { GetWindow(me, GW_HWNDPREV) };
    // (a few hundred at most: windows can come and go while they are being looked at)
    while w.0 != 0 && above.len() < 500 {
        if let Some(b) = Some(w).filter(|w| solid(*w)).and_then(bounds) {
            above.push(b);
        }
        w = unsafe { GetWindow(w, GW_HWNDPREV) };
    }
    covered_by(mine, above)
}

/// Where a window shows on the screen (its frame as drawn, without the invisible edges for resizing).
#[cfg(windows)]
pub(crate) fn bounds(w: windows::Win32::Foundation::HWND) -> Option<Area> {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
    use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;
    let mut r = RECT::default();
    let size = std::mem::size_of::<RECT>() as u32;
    let ok = unsafe { DwmGetWindowAttribute(w, DWMWA_EXTENDED_FRAME_BOUNDS, &mut r as *mut RECT as *mut _, size) }.is_ok()
        || unsafe { GetWindowRect(w, &mut r) }.as_bool();
    (ok && r.right > r.left && r.bottom > r.top).then_some((r.left, r.top, r.right, r.bottom))
}

/// A window that hides what is behind it: shown, not minimized, not hidden away (cloaked), and not see-through.
#[cfg(windows)]
fn solid(w: windows::Win32::Foundation::HWND) -> bool {
    use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetLayeredWindowAttributes, GetWindowLongW, IsIconic, IsWindowVisible, GWL_EXSTYLE, LAYERED_WINDOW_ATTRIBUTES_FLAGS,
        LWA_ALPHA, LWA_COLORKEY, WS_EX_LAYERED, WS_EX_TRANSPARENT,
    };
    unsafe {
        if !IsWindowVisible(w).as_bool() || IsIconic(w).as_bool() {
            return false;
        }
        let mut cloaked = 0u32;
        if DwmGetWindowAttribute(w, DWMWA_CLOAKED, &mut cloaked as *mut u32 as *mut _, 4).is_ok() && cloaked != 0 {
            return false;
        }
        let ex = GetWindowLongW(w, GWL_EXSTYLE) as u32;
        if ex & WS_EX_TRANSPARENT.0 != 0 {
            return false;
        }
        // a layered window is solid only when it says it is fully opaque (one drawn with its own see-through
        // pixels doesn't say at all)
        if ex & WS_EX_LAYERED.0 != 0 {
            let (mut alpha, mut flags) = (0u8, LAYERED_WINDOW_ATTRIBUTES_FLAGS(0));
            if !GetLayeredWindowAttributes(w, None, Some(&mut alpha as *mut u8), Some(&mut flags as *mut _)).as_bool() {
                return false;
            }
            if flags.0 & LWA_COLORKEY.0 != 0 || (flags.0 & LWA_ALPHA.0 != 0 && alpha < 255) {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_window_in_front_takes_its_share() {
        let mine = (0, 0, 100, 100);
        // one window over all of it, or bigger
        assert!(covered_by(mine, [(0, 0, 100, 100)]));
        assert!(covered_by(mine, [(-10, -10, 300, 300)]));
        // half, then the other half
        assert!(!covered_by(mine, [(0, 0, 50, 100)]));
        assert!(covered_by(mine, [(0, 0, 50, 100), (50, 0, 100, 100)]));
        // a hole in the middle is still seen
        assert!(!covered_by(mine, [(0, 0, 100, 40), (0, 60, 100, 100), (0, 40, 45, 60), (55, 40, 100, 60)]));
        // beside it, or nothing at all
        assert!(!covered_by(mine, [(100, 0, 200, 100)]));
        assert!(!covered_by(mine, []));
        // what is left after a cut is exactly what wasn't covered
        let left = cut(&[mine], (30, 30, 60, 60));
        let area: i64 = left.iter().map(|a| (a.2 - a.0) as i64 * (a.3 - a.1) as i64).sum();
        assert_eq!(area, 100 * 100 - 30 * 30);
    }

    /// BACKLINE_COVER_HWND=<window number>: is that window covered? (a live check on this computer's windows)
    #[cfg(windows)]
    #[test]
    #[ignore]
    fn covered_live() {
        let h: isize = std::env::var("BACKLINE_COVER_HWND").unwrap().parse().unwrap();
        println!("covered: {}", covered_window(windows::Win32::Foundation::HWND(h)));
    }
}
