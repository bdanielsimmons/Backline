//! Windows' media controls: Backline shows in the media flyout (and on the lock screen) with the song, its cover and
//! whether it is playing, and the media keys reach it through Windows - so they go to whichever player you used
//! last instead of being taken from the others. If the controls can't be set up, the media keys are caught as
//! hotkeys instead (the old way, which keeps them from other players).

use crate::Msg;
use eframe::egui;
use std::sync::mpsc::Sender;

#[cfg(windows)]
mod imp {
    use super::*;
    use std::ffi::c_void;

    #[repr(C)]
    struct RawMsg {
        hwnd: *mut c_void,
        message: u32,
        wparam: usize,
        lparam: isize,
        time: u32,
        pt_x: i32,
        pt_y: i32,
    }

    #[link(name = "user32")]
    extern "system" {
        fn RegisterHotKey(hwnd: *mut c_void, id: i32, modifiers: u32, vk: u32) -> i32;
        fn GetMessageW(msg: *mut RawMsg, hwnd: *mut c_void, min: u32, max: u32) -> i32;
    }

    const WM_HOTKEY: u32 = 0x0312;
    const MOD_NOREPEAT: u32 = 0x4000;

    pub fn spawn(tx: Sender<Msg>, ctx: egui::Context) {
        std::thread::spawn(move || unsafe {
            // id -> virtual key: 1 play/pause, 2 next, 3 previous, 4 stop
            for (id, vk) in [(1i32, 0xB3u32), (2, 0xB0), (3, 0xB1), (4, 0xB2)] {
                let ok = RegisterHotKey(std::ptr::null_mut(), id, MOD_NOREPEAT, vk);
                if ok == 0 {
                    crate::api::log(&format!("media key {} could not be registered (another app owns it?)", id));
                }
            }
            let mut m: RawMsg = std::mem::zeroed();
            while GetMessageW(&mut m, std::ptr::null_mut(), 0, 0) > 0 {
                if m.message == WM_HOTKEY {
                    let _ = tx.send(Msg::Media(m.wparam as u8));
                    ctx.request_repaint();
                }
            }
        });
    }
}

#[cfg(windows)]
pub fn spawn(tx: Sender<Msg>, ctx: egui::Context) {
    imp::spawn(tx, ctx);
}

#[cfg(not(windows))]
pub fn spawn(_tx: Sender<Msg>, _ctx: egui::Context) {}

/// What the media controls show: (artist, title, cover address) and 0 nothing, 1 playing, 2 paused.
pub type Shown = (Option<(String, String, String)>, u8);

/// Windows' media controls for Backline's window, and what they show now.
#[cfg(windows)]
pub struct Controls {
    c: souvlaki::MediaControls,
    shown: Shown,
}

#[cfg(not(windows))]
pub struct Controls;

/// Set up the media controls on the app's window: the buttons in the flyout and the media keys send the same
/// messages the hotkeys did (1 play/pause, 2 next, 3 previous, 4 stop). None when Windows won't have them.
#[cfg(windows)]
pub fn controls(cc: &eframe::CreationContext<'_>, tx: Sender<Msg>, ctx: egui::Context) -> Option<Controls> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let hwnd = match cc.window_handle().ok()?.as_raw() {
        RawWindowHandle::Win32(h) => h.hwnd.get() as *mut std::ffi::c_void,
        _ => return None,
    };
    HWND_VAL.store(hwnd as isize, std::sync::atomic::Ordering::Relaxed);
    let config = souvlaki::PlatformConfig { dbus_name: "backline", display_name: "Backline", hwnd: Some(hwnd) };
    let mut c = souvlaki::MediaControls::new(config).ok()?;
    c.attach(move |e: souvlaki::MediaControlEvent| {
        use souvlaki::MediaControlEvent as E;
        let id = match e {
            E::Toggle | E::Play | E::Pause => 1,
            E::Next => 2,
            E::Previous => 3,
            E::Stop => 4,
            _ => return,
        };
        let _ = tx.send(Msg::Media(id));
        ctx.request_repaint();
    })
    .ok()?;
    Some(Controls { c, shown: (None, 255) })
}

#[cfg(not(windows))]
pub fn controls(_cc: &eframe::CreationContext<'_>, _tx: Sender<Msg>, _ctx: egui::Context) -> Option<Controls> {
    None
}

/// Show the song and its state in the media controls (only when something changed).
#[cfg(windows)]
pub fn show(c: &mut Controls, now: Shown) {
    if c.shown == now {
        return;
    }
    if c.shown.0 != now.0 {
        match &now.0 {
            Some((artist, title, cover)) => {
                let _ = c.c.set_metadata(souvlaki::MediaMetadata {
                    title: Some(title),
                    artist: Some(artist),
                    cover_url: (!cover.is_empty()).then_some(cover.as_str()),
                    ..Default::default()
                });
            }
            None => {
                let _ = c.c.set_metadata(souvlaki::MediaMetadata { title: Some("Backline"), ..Default::default() });
            }
        }
    }
    let _ = c.c.set_playback(match now.1 {
        1 => souvlaki::MediaPlayback::Playing { progress: None },
        2 => souvlaki::MediaPlayback::Paused { progress: None },
        _ => souvlaki::MediaPlayback::Stopped,
    });
    c.shown = now;
}

#[cfg(not(windows))]
pub fn show(_c: &mut Controls, _now: Shown) {}

// ------------------------------------------------------------------ taskbar buttons
// Previous, play / pause and next under the window's thumbnail when you hover its taskbar button.

/// The app window, kept when the media controls are set up (the taskbar buttons go on it too).
#[cfg(windows)]
static HWND_VAL: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);
/// Where a click on a taskbar button is sent.
#[cfg(windows)]
static THUMB_TX: std::sync::Mutex<Option<(Sender<Msg>, egui::Context)>> = std::sync::Mutex::new(None);

/// The buttons under the taskbar thumbnail, and whether they show play or pause.
#[cfg(windows)]
pub struct ThumbBar {
    tb: windows::Win32::UI::Shell::ITaskbarList3,
    hwnd: windows::Win32::Foundation::HWND,
    icons: [windows::Win32::UI::WindowsAndMessaging::HICON; 4],
    playing: Option<bool>,
}

#[cfg(not(windows))]
pub struct ThumbBar;

/// A 16 x 16 icon drawn from rows of '#' (white) and '.' (clear).
#[cfg(windows)]
fn icon(rows: &[&str; 16]) -> windows::Win32::UI::WindowsAndMessaging::HICON {
    use windows::Win32::UI::WindowsAndMessaging::CreateIcon;
    let mut color = Vec::with_capacity(16 * 16 * 4);
    let mut mask = vec![0u8; 16 * 2];
    for (y, row) in rows.iter().enumerate() {
        for (x, ch) in row.chars().enumerate().take(16) {
            let on = ch == '#';
            color.extend_from_slice(if on { &[255, 255, 255, 255] } else { &[0, 0, 0, 0] });
            if !on {
                mask[y * 2 + x / 8] |= 0x80 >> (x % 8);
            }
        }
    }
    unsafe { CreateIcon(None, 16, 16, 1, 32, mask.as_ptr(), color.as_ptr()).unwrap_or_default() }
}

#[cfg(windows)]
const ICONS: [[&str; 16]; 4] = [
    // previous
    [
        "................",
        "................",
        "................",
        "...##......##...",
        "...##.....###...",
        "...##....####...",
        "...##...#####...",
        "...##..######...",
        "...##..######...",
        "...##...#####...",
        "...##....####...",
        "...##.....###...",
        "...##......##...",
        "................",
        "................",
        "................",
    ],
    // play
    [
        "................",
        "................",
        "....#...........",
        "....##..........",
        "....###.........",
        "....####........",
        "....#####.......",
        "....######......",
        "....######......",
        "....#####.......",
        "....####........",
        "....###.........",
        "....##..........",
        "....#...........",
        "................",
        "................",
    ],
    // pause
    [
        "................",
        "................",
        "................",
        "....###..###....",
        "....###..###....",
        "....###..###....",
        "....###..###....",
        "....###..###....",
        "....###..###....",
        "....###..###....",
        "....###..###....",
        "....###..###....",
        "....###..###....",
        "................",
        "................",
        "................",
    ],
    // next
    [
        "................",
        "................",
        "................",
        "...##......##...",
        "...###.....##...",
        "...####....##...",
        "...#####...##...",
        "...######..##...",
        "...######..##...",
        "...#####...##...",
        "...####....##...",
        "...###.....##...",
        "...##......##...",
        "................",
        "................",
        "................",
    ],
];

/// A click on a taskbar button arrives at the window as WM_COMMAND: send it on like a media key.
#[cfg(windows)]
unsafe extern "system" fn thumb_proc(
    hwnd: windows::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
    _id: usize,
    _data: usize,
) -> windows::Win32::Foundation::LRESULT {
    const WM_COMMAND: u32 = 0x0111;
    const THBN_CLICKED: usize = 0x1800;
    if msg == WM_COMMAND && (wparam.0 >> 16) & 0xFFFF == THBN_CLICKED {
        let id = (wparam.0 & 0xFFFF) as u8;
        if let Some((tx, ctx)) = THUMB_TX.lock().ok().and_then(|g| g.clone()) {
            let _ = tx.send(Msg::Media(id));
            ctx.request_repaint();
        }
        return windows::Win32::Foundation::LRESULT(0);
    }
    windows::Win32::UI::Shell::DefSubclassProc(hwnd, msg, wparam, lparam)
}

/// Put the buttons under the taskbar thumbnail (once the taskbar has made its button for the window).
#[cfg(windows)]
pub fn thumb_bar(tx: Sender<Msg>, ctx: egui::Context) -> Option<ThumbBar> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED};
    use windows::Win32::UI::Shell::{ITaskbarList3, SetWindowSubclass, TaskbarList};
    let hwnd = HWND(HWND_VAL.load(std::sync::atomic::Ordering::Relaxed));
    if hwnd.0 == 0 {
        return None;
    }
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let tb: ITaskbarList3 = CoCreateInstance(&TaskbarList, None, CLSCTX_INPROC_SERVER).ok()?;
        tb.HrInit().ok()?;
        *THUMB_TX.lock().ok()? = Some((tx, ctx));
        SetWindowSubclass(hwnd, Some(thumb_proc), 0xB4C1, 0);
        let icons = [icon(&ICONS[0]), icon(&ICONS[1]), icon(&ICONS[2]), icon(&ICONS[3])];
        let buttons = [button(3, icons[0], "Previous"), button(1, icons[1], "Play"), button(2, icons[3], "Next")];
        tb.ThumbBarAddButtons(hwnd, &buttons).ok()?;
        Some(ThumbBar { tb, hwnd, icons, playing: None })
    }
}

#[cfg(not(windows))]
pub fn thumb_bar(_tx: Sender<Msg>, _ctx: egui::Context) -> Option<ThumbBar> {
    None
}

#[cfg(windows)]
fn button(id: u32, icon: windows::Win32::UI::WindowsAndMessaging::HICON, tip: &str) -> windows::Win32::UI::Shell::THUMBBUTTON {
    use windows::Win32::UI::Shell::{THBF_ENABLED, THB_FLAGS, THB_ICON, THB_TOOLTIP, THUMBBUTTON, THUMBBUTTONMASK};
    let mut sz = [0u16; 260];
    for (i, c) in tip.encode_utf16().take(259).enumerate() {
        sz[i] = c;
    }
    THUMBBUTTON {
        dwMask: THUMBBUTTONMASK(THB_ICON.0 | THB_TOOLTIP.0 | THB_FLAGS.0),
        iId: id,
        iBitmap: 0,
        hIcon: icon,
        szTip: sz,
        dwFlags: THBF_ENABLED,
    }
}

/// The middle button shows pause while something plays, play otherwise.
#[cfg(windows)]
pub fn thumb_show(t: &mut ThumbBar, playing: bool) {
    if t.playing == Some(playing) {
        return;
    }
    let b = if playing { button(1, t.icons[2], "Pause") } else { button(1, t.icons[1], "Play") };
    unsafe {
        let _ = t.tb.ThumbBarUpdateButtons(t.hwnd, &[b]);
    }
    t.playing = Some(playing);
}

#[cfg(not(windows))]
pub fn thumb_show(_t: &mut ThumbBar, _playing: bool) {}
