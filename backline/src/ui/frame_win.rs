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

/// The frame is drawn: its title-bar parts are the ones in force, and whether the skin's bars are the title bar.
pub(crate) fn end_frame(on: bool) {
    if let (Ok(mut next), Ok(mut cap)) = (NEXT.lock(), CAPTION.lock()) {
        *cap = std::mem::take(&mut *next);
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
            let on_bar = CAPTION.lock().is_ok_and(|v| v.iter().any(|c| x >= c[0] && x < c[2] && y >= c[1] && y < c[3]));
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
