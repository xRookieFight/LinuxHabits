use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering};

use windows::core::Result;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetAncestor, GetClassNameW, PostMessageW, PostThreadMessageW,
    SetWindowsHookExW, WindowFromPoint, GA_ROOT, HC_ACTION, HHOOK, MSLLHOOKSTRUCT, WH_MOUSE_LL,
    WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL,
};

use crate::{config, osd, tray_probe};

// the hook proc is a bare extern fn, so it can only reach these through statics
static OSD_HWND: AtomicIsize = AtomicIsize::new(0);
static PROBE_THREAD: AtomicU32 = AtomicU32::new(0);
static MIDDLE_SWALLOWED: AtomicBool = AtomicBool::new(false);

/// Installs the global low level mouse hook. It runs on the calling thread,
/// so that thread has to keep pumping messages.
pub fn install(osd: HWND, probe_thread: u32) -> Result<HHOOK> {
    OSD_HWND.store(osd.0 as isize, Ordering::Relaxed);
    PROBE_THREAD.store(probe_thread, Ordering::Relaxed);
    unsafe {
        let module = GetModuleHandleW(None)?;
        SetWindowsHookExW(WH_MOUSE_LL, Some(hook_proc), module, 0)
    }
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let info = &*(lparam.0 as *const MSLLHOOKSTRUCT);
        match wparam.0 as u32 {
            WM_MOUSEMOVE => on_move(info.pt),
            WM_MOUSEWHEEL if on_wheel(info) => return LRESULT(1),
            WM_MBUTTONDOWN if on_middle_down(info.pt) => return LRESULT(1),
            // the up has to be eaten too, otherwise the taskbar sees a release it never got a press for
            WM_MBUTTONUP if MIDDLE_SWALLOWED.swap(false, Ordering::Relaxed) => return LRESULT(1),
            _ => {}
        }
    }
    CallNextHookEx(HHOOK::default(), code, wparam, lparam)
}

unsafe fn on_middle_down(pt: POINT) -> bool {
    if !config::volume_middle_click_mute() || !over_taskbar(pt) || !tray_probe::volume_icon_contains(pt) {
        return false;
    }
    MIDDLE_SWALLOWED.store(true, Ordering::Relaxed);
    let osd = HWND(OSD_HWND.load(Ordering::Relaxed) as _);
    let _ = PostMessageW(osd, osd::WM_VOLUME_TOGGLE_MUTE, WPARAM(0), LPARAM(0));
    true
}

unsafe fn on_move(pt: POINT) {
    let wanted = config::volume_scroll() || config::volume_middle_click_mute();
    if !wanted || !over_taskbar(pt) {
        tray_probe::clear();
        return;
    }
    let _ = PostThreadMessageW(
        PROBE_THREAD.load(Ordering::Relaxed),
        tray_probe::WM_PROBE,
        WPARAM(pt.x as isize as usize),
        LPARAM(pt.y as isize),
    );
}

// returns true when the wheel event is ours and should not reach the taskbar
unsafe fn on_wheel(info: &MSLLHOOKSTRUCT) -> bool {
    if !config::volume_scroll() || !over_taskbar(info.pt) || !tray_probe::volume_icon_contains(info.pt) {
        return false;
    }
    let delta = (info.mouseData >> 16) as i16;
    let osd = HWND(OSD_HWND.load(Ordering::Relaxed) as _);
    let _ = PostMessageW(
        osd,
        osd::WM_VOLUME_WHEEL,
        WPARAM(delta as isize as usize),
        LPARAM(0),
    );
    true
}

unsafe fn over_taskbar(pt: POINT) -> bool {
    let root = GetAncestor(WindowFromPoint(pt), GA_ROOT);
    if root.0.is_null() {
        return false;
    }
    let mut buf = [0u16; 32];
    let len = GetClassNameW(root, &mut buf) as usize;
    let class = String::from_utf16_lossy(&buf[..len]);
    class == "Shell_TrayWnd" || class == "Shell_SecondaryTrayWnd"
}
