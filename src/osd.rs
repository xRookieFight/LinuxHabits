use std::cell::{Cell, OnceCell};

use windows::core::{w, Result, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontIndirectW,
    CreateRoundRectRgn, CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW, EndPaint, FillRect,
    GetMonitorInfoW, InvalidateRect, MonitorFromPoint, SelectObject, SetBkMode, SetTextColor,
    SetWindowRgn, CLEARTYPE_QUALITY, DRAW_TEXT_FORMAT, DT_CENTER, DT_RIGHT, DT_SINGLELINE,
    DT_VCENTER, FW_NORMAL, FW_SEMIBOLD, HDC, LOGFONTW, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    PAINTSTRUCT, SRCCOPY, TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, GetClientRect, GetCursorPos, KillTimer, PostQuitMessage,
    RegisterClassW,
    SetLayeredWindowAttributes, SetTimer, SetWindowPos, ShowWindow, HWND_TOPMOST, LWA_ALPHA,
    SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_HIDE, WM_APP, WM_ERASEBKGND, WM_PAINT, WM_TIMER,
    WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_EX_TRANSPARENT, WS_POPUP,
};

use crate::audio::{Audio, VolumeState};
use crate::config;

/// Posted by the mouse hook, wParam carries the raw wheel delta as a signed value.
pub const WM_VOLUME_WHEEL: u32 = WM_APP + 2;
/// Posted by the mouse hook on a middle click over the volume icon.
pub const WM_VOLUME_TOGGLE_MUTE: u32 = WM_APP + 3;

const STEP: f32 = 0.02;
const WHEEL_DELTA: i32 = 120;

// sizes are in 96 dpi pixels and get scaled per monitor
const WIDTH: f32 = 320.0;
const HEIGHT: f32 = 72.0;
const RADIUS: f32 = 16.0;

const VISIBLE_MS: u32 = 1200;
const FADE_TICK_MS: u32 = 15;
const FADE_STEP: u8 = 20;
const MAX_ALPHA: u8 = 235;

const TIMER_HIDE: usize = 1;
const TIMER_FADE: usize = 2;

// COLORREF is 0x00BBGGRR
const BG: COLORREF = COLORREF(0x002b_2b2b);
const FG: COLORREF = COLORREF(0x00ff_ffff);
const TRACK: COLORREF = COLORREF(0x0050_5050);
const ACCENT: COLORREF = COLORREF(0x00ff_c24c);
const MUTED: COLORREF = COLORREF(0x008a_8a8a);

// everything below lives on the UI thread only. Cells instead of a RefCell because
// ShowWindow/SetWindowRgn can re-enter the wndproc while we're still in a handler
thread_local! {
    static AUDIO: OnceCell<Audio> = const { OnceCell::new() };
    static STATE: Cell<VolumeState> = Cell::new(VolumeState::default());
    static WHEEL_ACC: Cell<i32> = const { Cell::new(0) };
    static ALPHA: Cell<u8> = const { Cell::new(0) };
    static SCALE: Cell<f32> = const { Cell::new(1.0) };
}

/// Creates the hidden, click-through OSD window. It also owns the volume changes,
/// since the hook just forwards wheel deltas to it.
pub fn create() -> Result<HWND> {
    let audio = Audio::new()?;
    AUDIO.with(|cell| {
        let _ = cell.set(audio);
    });

    unsafe {
        let instance = GetModuleHandleW(None)?;
        // the settings app finds the daemon by this class name, keep them in sync
        let class = w!("LinuxHabitsOsd");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: instance.into(),
            lpszClassName: class,
            ..Default::default()
        };
        RegisterClassW(&wc);

        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
            class,
            w!("LinuxHabits"),
            WS_POPUP,
            0,
            0,
            0,
            0,
            None,
            None,
            instance,
            None,
        )?;
        SetLayeredWindowAttributes(hwnd, COLORREF(0), 0, LWA_ALPHA)?;
        Ok(hwnd)
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_VOLUME_WHEEL => {
            on_wheel(hwnd, wparam.0 as isize as i32);
            LRESULT(0)
        }
        WM_VOLUME_TOGGLE_MUTE => {
            let state = AUDIO.with(|cell| cell.get().and_then(|a| a.toggle_mute().ok()));
            if let Some(state) = state {
                STATE.set(state);
                show(hwnd);
            }
            LRESULT(0)
        }
        WM_TIMER => {
            on_timer(hwnd, wparam.0);
            LRESULT(0)
        }
        WM_PAINT => {
            paint(hwnd);
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        // registered messages aren't constants, so they can't be match arms
        _ if msg == config::reload_message() => {
            config::load();
            LRESULT(0)
        }
        _ if msg == config::quit_message() => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn on_wheel(hwnd: HWND, delta: i32) {
    // precision touchpads and free spinning wheels send tiny deltas, so collect them into notches
    let acc = WHEEL_ACC.get() + delta;
    let notches = acc / WHEEL_DELTA;
    WHEEL_ACC.set(acc % WHEEL_DELTA);
    if notches == 0 {
        return;
    }

    let state = AUDIO.with(|cell| cell.get().and_then(|a| a.step(notches as f32 * STEP).ok()));
    if let Some(state) = state {
        STATE.set(state);
        show(hwnd);
    }
}

unsafe fn show(hwnd: HWND) {
    let mut cursor = POINT::default();
    let _ = GetCursorPos(&mut cursor);
    let monitor = MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST);

    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    let _ = GetMonitorInfoW(monitor, &mut info);
    let (mut dpi_x, mut dpi_y) = (96, 96);
    let _ = GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);
    let scale = dpi_x as f32 / 96.0;
    SCALE.set(scale);

    let w = (WIDTH * scale).round() as i32;
    let h = (HEIGHT * scale).round() as i32;
    let r = (RADIUS * scale).round() as i32;
    let area = info.rcMonitor;
    let x = area.left + (area.right - area.left - w) / 2;
    let y = area.top + (area.bottom - area.top - h) / 2;

    // the system owns the region after this call, no need to free it
    SetWindowRgn(hwnd, CreateRoundRectRgn(0, 0, w + 1, h + 1, r, r), false);
    ALPHA.set(MAX_ALPHA);
    let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), MAX_ALPHA, LWA_ALPHA);
    let _ = SetWindowPos(hwnd, HWND_TOPMOST, x, y, w, h, SWP_NOACTIVATE | SWP_SHOWWINDOW);
    let _ = InvalidateRect(hwnd, None, false);

    // each scroll restarts the countdown
    let _ = KillTimer(hwnd, TIMER_FADE);
    SetTimer(hwnd, TIMER_HIDE, VISIBLE_MS, None);
}

unsafe fn on_timer(hwnd: HWND, id: usize) {
    match id {
        TIMER_HIDE => {
            let _ = KillTimer(hwnd, TIMER_HIDE);
            SetTimer(hwnd, TIMER_FADE, FADE_TICK_MS, None);
        }
        TIMER_FADE => {
            let alpha = ALPHA.get().saturating_sub(FADE_STEP);
            ALPHA.set(alpha);
            if alpha == 0 {
                let _ = KillTimer(hwnd, TIMER_FADE);
                let _ = ShowWindow(hwnd, SW_HIDE);
            } else {
                let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), alpha, LWA_ALPHA);
            }
        }
        _ => {}
    }
}

unsafe fn paint(hwnd: HWND) {
    let mut ps = PAINTSTRUCT::default();
    let hdc = BeginPaint(hwnd, &mut ps);
    let mut rc = RECT::default();
    let _ = GetClientRect(hwnd, &mut rc);

    // draw offscreen first, painting straight to the window flickers while scrolling fast
    let mem = CreateCompatibleDC(hdc);
    let bitmap = CreateCompatibleBitmap(hdc, rc.right, rc.bottom);
    let old = SelectObject(mem, bitmap);
    draw(mem, rc.right, rc.bottom, STATE.get(), SCALE.get());
    let _ = BitBlt(hdc, 0, 0, rc.right, rc.bottom, mem, 0, 0, SRCCOPY);
    SelectObject(mem, old);
    let _ = DeleteObject(bitmap);
    let _ = DeleteDC(mem);

    let _ = EndPaint(hwnd, &ps);
}

unsafe fn draw(hdc: HDC, w: i32, h: i32, state: VolumeState, scale: f32) {
    let px = |v: f32| (v * scale).round() as i32;

    fill(hdc, RECT { left: 0, top: 0, right: w, bottom: h }, BG);
    SetBkMode(hdc, TRANSPARENT);

    // Segoe Fluent Icons speaker glyphs, ships with Windows 11
    let glyph = match (state.muted, state.percent) {
        (true, _) | (false, 0) => "\u{E74F}",
        (false, 1..=33) => "\u{E993}",
        (false, 34..=66) => "\u{E994}",
        _ => "\u{E995}",
    };
    let icon_rect = RECT { left: px(16.0), top: 0, right: px(56.0), bottom: h };
    text(hdc, glyph, icon_rect, w!("Segoe Fluent Icons"), px(24.0), FW_NORMAL.0 as i32, DT_CENTER);

    let number_rect = RECT { left: w - px(64.0), top: 0, right: w - px(18.0), bottom: h };
    let number = state.percent.to_string();
    text(hdc, &number, number_rect, w!("Segoe UI"), px(22.0), FW_SEMIBOLD.0 as i32, DT_RIGHT);

    let bar_h = px(6.0).max(2);
    let bar = RECT {
        left: px(68.0),
        top: (h - bar_h) / 2,
        right: w - px(76.0),
        bottom: (h - bar_h) / 2 + bar_h,
    };
    fill(hdc, bar, TRACK);
    let filled = RECT {
        right: bar.left + (bar.right - bar.left) * state.percent as i32 / 100,
        ..bar
    };
    fill(hdc, filled, if state.muted { MUTED } else { ACCENT });
}

unsafe fn fill(hdc: HDC, rect: RECT, color: COLORREF) {
    let brush = CreateSolidBrush(color);
    FillRect(hdc, &rect, brush);
    let _ = DeleteObject(brush);
}

unsafe fn text(
    hdc: HDC,
    s: &str,
    mut rect: RECT,
    face: PCWSTR,
    height: i32,
    weight: i32,
    align: DRAW_TEXT_FORMAT,
) {
    let mut font = LOGFONTW {
        lfHeight: -height,
        lfWeight: weight,
        lfQuality: CLEARTYPE_QUALITY,
        ..Default::default()
    };
    let face = face.as_wide();
    let len = face.len().min(font.lfFaceName.len() - 1);
    font.lfFaceName[..len].copy_from_slice(&face[..len]);

    let hfont = CreateFontIndirectW(&font);
    let old = SelectObject(hdc, hfont);
    SetTextColor(hdc, FG);
    let mut wide: Vec<u16> = s.encode_utf16().collect();
    DrawTextW(hdc, &mut wide, &mut rect, align | DT_VCENTER | DT_SINGLELINE);
    SelectObject(hdc, old);
    let _ = DeleteObject(hfont);
}
