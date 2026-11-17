#![windows_subsystem = "windows"]

mod audio;
mod config;
mod mouse_hook;
mod osd;
mod tray_probe;

use windows::core::{w, Result};
use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, TranslateMessage, UnhookWindowsHookEx, MSG,
};

fn main() -> Result<()> {
    unsafe {
        // hook coords, UIA rects and monitor rects all need to be in physical pixels
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

        let _instance = CreateMutexW(None, true, w!("Local\\LinuxHabits"))?;
        if GetLastError() == ERROR_ALREADY_EXISTS {
            return Ok(());
        }

        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
        config::load();

        let probe_thread = tray_probe::spawn();
        let osd = osd::create()?;
        let hook = mouse_hook::install(osd, probe_thread)?;

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        UnhookWindowsHookEx(hook)?;
    }
    Ok(())
}
