//! Windows: when the skin's bars are the window's title bar, Windows itself is told so (WM_NCHITTEST). Pressing one is
//! pressing a title bar: dragging moves the window with Snap (half the screen, a corner, the top to fill it),
//! double-click fills the screen or brings it back, right-click shows the window's menu; the window's edges size it with
//! the system's own pointer. A window filling the screen fills just its work area, the taskbar kept clear and nothing
//! hanging past the edge (WM_GETMINMAXINFO). The parts are noted each frame (`caption`, `end_frame`); the window's own
//! messages are read in `proc_`, ahead of winit's.

use eframe::egui;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::Mutex;

/// The title-bar parts noted this frame and the ones in force (physical pixels, the window's own coordinates).
static NEXT: Mutex<Vec<[f32; 4]>> = Mutex::new(Vec::new());
static CAPTION: Mutex<Vec<[f32; 4]>> = Mutex::new(Vec::new());
/// The skin's bars are the title bar now (the full window wearing a skin, not full screen).
static ON: AtomicBool = AtomicBool::new(false);
static HWND_V: AtomicIsize = AtomicIsize::new(0);

/// Whether Windows is handling the title bar and the edges (else the app does it itself: `app_bar_strip`).
pub(crate) fn active() -> bool {
    ON.load(Ordering::Relaxed) && HWND_V.load(Ordering::Relaxed) != 0
}

/// A part of the title bar this frame (points; `ppp` the points' size in pixels).
pub(crate) fn caption(r: egui::Rect, ppp: f32) {
    if let Ok(mut v) = NEXT.lock() {
        v.push([r.min.x * ppp, r.min.y * ppp, r.max.x * ppp, r.max.y * ppp]);
    }
}

/// Things drawn over the title bar that take their own clicks (the update bar, a menu, a tip, a dialog): noted this
/// frame and in force, as the title-bar parts are; over one of them the window is the app's, not the title bar.
static HOLES_NEXT: Mutex<Vec<[f32; 4]>> = Mutex::new(Vec::new());
static HOLES: Mutex<Vec<[f32; 4]>> = Mutex::new(Vec::new());

/// Something floating over the title bar this frame (points), clickable in its own right.
pub(crate) fn hole(r: egui::Rect, ppp: f32) {
    if let Ok(mut v) = HOLES_NEXT.lock() {
        v.push([r.min.x * ppp, r.min.y * ppp, r.max.x * ppp, r.max.y * ppp]);
    }
}

/// The frame is drawn: its title-bar parts are the ones in force, and whether the skin's bars are the title bar.
pub(crate) fn end_frame(on: bool) {
    if let (Ok(mut next), Ok(mut cap)) = (NEXT.lock(), CAPTION.lock()) {
        *cap = std::mem::take(&mut *next);
    }
    if let (Ok(mut next), Ok(mut holes)) = (HOLES_NEXT.lock(), HOLES.lock()) {
        *holes = std::mem::take(&mut *next);
    }
    ON.store(on, Ordering::Relaxed);
}

#[cfg(windows)]
pub(crate) fn install(cc: &eframe::CreationContext<'_>) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Shell::SetWindowSubclass;
    let Some(h) = cc.window_handle().ok().and_then(|h| match h.as_raw() {
        RawWindowHandle::Win32(w) => Some(w.hwnd.get()),
        _ => None,
    }) else {
        return;
    };
    if unsafe { SetWindowSubclass(HWND(h), Some(proc_), 0xB4C1, 0) }.as_bool() {
        HWND_V.store(h, Ordering::Relaxed);
    }
}

#[cfg(not(windows))]
pub(crate) fn install(_cc: &eframe::CreationContext<'_>) {}

/// The main screen's work area (the taskbar kept clear), in points: where the window first opens.
#[cfg(windows)]
pub(crate) fn work_area() -> Option<egui::Rect> {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::HiDpi::GetDpiForSystem;
    use windows::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETWORKAREA, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS};
    let mut r = RECT::default();
    let ok = unsafe { SystemParametersInfoW(SPI_GETWORKAREA, 0, Some(&mut r as *mut RECT as *const _), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0)) }
        .as_bool();
    // (pixels once the program knows the screen's scale; before that Windows answers in points, and says 96 here)
    let k = 96.0 / unsafe { GetDpiForSystem() }.max(96) as f32;
    (ok && r.right > r.left && r.bottom > r.top).then(|| {
        egui::Rect::from_min_max(egui::pos2(r.left as f32 * k, r.top as f32 * k), egui::pos2(r.right as f32 * k, r.bottom as f32 * k))
    })
}

#[cfg(not(windows))]
pub(crate) fn work_area() -> Option<egui::Rect> {
    None
}

/// The window kept on its screen: moved in when it hangs past an edge of the screen's work area (the taskbar kept
/// clear), and made smaller when it's bigger than that, so nothing at its bottom (the tour's card, the player) is out
/// of reach. Not while it fills the screen or is minimized.
#[cfg(windows)]
pub(crate) fn fit_on_screen(h: isize) {
    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST};
    use windows::Win32::UI::WindowsAndMessaging::{GetWindowRect, IsIconic, IsZoomed, SetWindowPos, SWP_NOACTIVATE, SWP_NOZORDER};
    let h = HWND(h);
    if h.0 == 0 || unsafe { IsZoomed(h).as_bool() || IsIconic(h).as_bool() } {
        return;
    }
    let mut wr = RECT::default();
    if !unsafe { GetWindowRect(h, &mut wr) }.as_bool() {
        return;
    }
    // (what shows of it: a framed window's edges for sizing are see-through)
    let (l, t, r, b) = crate::onscreen::bounds(h).unwrap_or((wr.left, wr.top, wr.right, wr.bottom));
    let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
    if !unsafe { GetMonitorInfoW(MonitorFromWindow(h, MONITOR_DEFAULTTONEAREST), &mut mi) }.as_bool() {
        return;
    }
    let wa = mi.rcWork;
    let Some((x, y, w, ht)) = fit_into((l, t, r, b), (wa.left, wa.top, wa.right, wa.bottom)) else {
        return;
    };
    // (back to the window's own rectangle: the see-through edges round what shows)
    let (el, et, er, eb) = (l - wr.left, t - wr.top, wr.right - r, wr.bottom - b);
    unsafe { SetWindowPos(h, HWND(0), x - el, y - et, w + el + er, ht + et + eb, SWP_NOZORDER | SWP_NOACTIVATE) };
    crate::api::log(&format!("window: moved onto the screen ({}x{} at {},{}; work area {}x{})", w, ht, x, y, wa.right - wa.left, wa.bottom - wa.top));
}

#[cfg(not(windows))]
pub(crate) fn fit_on_screen(_h: isize) {}

/// Where a window (left, top, right, bottom) goes to be all on a work area (the same): no bigger than it, moved in
/// from any edge it hangs past. Left, top, width, height; None when it's on it already.
fn fit_into(win: (i32, i32, i32, i32), wa: (i32, i32, i32, i32)) -> Option<(i32, i32, i32, i32)> {
    let (l, t, r, b) = win;
    let (w, h) = ((r - l).min(wa.2 - wa.0), (b - t).min(wa.3 - wa.1));
    let (x, y) = (l.min(wa.2 - w).max(wa.0), t.min(wa.3 - h).max(wa.1));
    ((x, y, w, h) != (l, t, r - l, b - t)).then_some((x, y, w, h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_window_past_the_screen_comes_onto_it() {
        // a 1920 x 1080 screen at 150%, the taskbar 48 high: a window 1920 x 1230 from the top runs past the bottom
        let wa = (0, 0, 1920, 1032);
        assert_eq!(fit_into((0, 0, 1920, 1230), wa), Some((0, 0, 1920, 1032)));
        // opened lower down: up, and no taller than the work area
        assert_eq!(fit_into((100, 150, 1300, 1250), wa), Some((100, 0, 1200, 1032)));
        // hanging past the bottom but short enough: just moved up
        assert_eq!(fit_into((100, 600, 1300, 1100), wa), Some((100, 532, 1200, 500)));
        // past the left, and on a second screen to the right
        assert_eq!(fit_into((-50, 10, 950, 610), wa), Some((0, 10, 1000, 600)));
        assert_eq!(fit_into((1800, 100, 2800, 700), (1920, 0, 3840, 1040)), Some((1920, 100, 1000, 600)));
        // already on it: left be
        assert_eq!(fit_into((100, 100, 1100, 900), wa), None);
        assert_eq!(fit_into((0, 0, 1920, 1032), wa), None);
    }

    #[cfg(windows)]
    #[test]
    fn the_work_area_is_found() {
        let wa = work_area().expect("the work area");
        assert!(wa.width() >= 640.0 && wa.height() >= 400.0, "{wa:?}");
    }
}

#[cfg(windows)]
unsafe extern "system" fn proc_(
    hwnd: windows::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
    _id: usize,
    _data: usize,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::Foundation::{LRESULT, POINT, RECT};
    use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, ScreenToClient, MONITORINFO, MONITOR_DEFAULTTONEAREST};
    use windows::Win32::UI::HiDpi::GetDpiForWindow;
    use windows::Win32::UI::Shell::DefSubclassProc;
    use windows::Win32::UI::WindowsAndMessaging::{GetClientRect, IsZoomed, MINMAXINFO};
    const WM_GETMINMAXINFO: u32 = 0x0024;
    const WM_NCHITTEST: u32 = 0x0084;
    if !ON.load(Ordering::Relaxed) {
        return DefSubclassProc(hwnd, msg, wparam, lparam);
    }
    match msg {
        WM_NCHITTEST => {
            let mut pt = POINT { x: (lparam.0 & 0xFFFF) as i16 as i32, y: ((lparam.0 >> 16) & 0xFFFF) as i16 as i32 };
            if !ScreenToClient(hwnd, &mut pt).as_bool() {
                return DefSubclassProc(hwnd, msg, wparam, lparam);
            }
            let (x, y) = (pt.x as f32, pt.y as f32);
            // the edges: sizing, as a framed window's (not while it fills the screen)
            if !IsZoomed(hwnd).as_bool() {
                let mut rc = RECT::default();
                GetClientRect(hwnd, &mut rc);
                let m = 6.0 * GetDpiForWindow(hwnd).max(96) as f32 / 96.0;
                let (l, r, t, b) = (x < m, x >= rc.right as f32 - m, y < m, y >= rc.bottom as f32 - m);
                let hit = match (l, r, t, b) {
                    (true, _, true, _) => 13, // HTTOPLEFT
                    (_, true, true, _) => 14, // HTTOPRIGHT
                    (true, _, _, true) => 16, // HTBOTTOMLEFT
                    (_, true, _, true) => 17, // HTBOTTOMRIGHT
                    (true, ..) => 10,         // HTLEFT
                    (_, true, ..) => 11,      // HTRIGHT
                    (_, _, true, _) => 12,    // HTTOP
                    (.., true) => 15,         // HTBOTTOM
                    _ => 0,
                };
                if hit != 0 {
                    return LRESULT(hit);
                }
            }
            let inside = |c: &[f32; 4]| x >= c[0] && x < c[2] && y >= c[1] && y < c[3];
            // (something floating over the bar - the update bar, a menu - takes its own clicks)
            let over = HOLES.lock().is_ok_and(|v| v.iter().any(inside));
            let on_bar = !over && CAPTION.lock().is_ok_and(|v| v.iter().any(inside));
            if on_bar {
                return LRESULT(2); // HTCAPTION
            }
            DefSubclassProc(hwnd, msg, wparam, lparam)
        }
        WM_GETMINMAXINFO => {
            let r = DefSubclassProc(hwnd, msg, wparam, lparam);
            // filling the screen: just its work area (the taskbar stays clear, nothing past the edge)
            let mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
            let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
            if GetMonitorInfoW(mon, &mut mi).as_bool() {
                let info = &mut *(lparam.0 as *mut MINMAXINFO);
                let (w, m) = (mi.rcWork, mi.rcMonitor);
                info.ptMaxPosition = POINT { x: w.left - m.left, y: w.top - m.top };
                info.ptMaxSize = POINT { x: w.right - w.left, y: w.bottom - w.top };
            }
            r
        }
        _ => DefSubclassProc(hwnd, msg, wparam, lparam),
    }
}
