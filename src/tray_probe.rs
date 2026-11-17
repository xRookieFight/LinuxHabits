use std::sync::{mpsc, Mutex};
use std::thread;

use windows::core::Result;
use windows::Win32::Foundation::{POINT, RECT};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Accessibility::{CUIAutomation, IUIAutomation};
use windows::Win32::UI::WindowsAndMessaging::{
    GetMessageW, PeekMessageW, MSG, PM_NOREMOVE, PM_REMOVE, WM_APP, WM_USER,
};

use crate::audio::Audio;

/// Posted to the probe thread with the cursor position packed into wParam/lParam.
pub const WM_PROBE: u32 = WM_APP + 1;

// every tray button on the Win11 taskbar shares this id, the name is what tells them apart
const TRAY_BUTTON_ID: &str = "SystemTrayIcon";

static VOLUME_RECT: Mutex<Option<RECT>> = Mutex::new(None);

/// True if the last probe found the volume icon and `pt` is inside it.
/// Cheap enough to call from the mouse hook.
pub fn volume_icon_contains(pt: POINT) -> bool {
    VOLUME_RECT.lock().unwrap().is_some_and(|r| {
        pt.x >= r.left && pt.x < r.right && pt.y >= r.top && pt.y < r.bottom
    })
}

pub fn clear() {
    *VOLUME_RECT.lock().unwrap() = None;
}

/// Starts the UI Automation thread and returns its thread id for PostThreadMessage.
/// UIA calls go into explorer and can take a while, so they must never run inside the hook.
pub fn spawn() -> u32 {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || unsafe { run(tx) });
    rx.recv().unwrap_or(0)
}

unsafe fn run(ready: mpsc::Sender<u32>) {
    let mut msg = MSG::default();
    // ok so the thread has no message queue until it touches one, and posts before that just vanish
    let _ = PeekMessageW(&mut msg, None, WM_USER, WM_USER, PM_NOREMOVE);
    let _ = ready.send(GetCurrentThreadId());

    if CoInitializeEx(None, COINIT_MULTITHREADED).is_err() {
        return;
    }
    let Ok(automation) =
        CoCreateInstance::<_, IUIAutomation>(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
    else {
        return;
    };
    let Ok(audio) = Audio::new() else { return };

    while GetMessageW(&mut msg, None, 0, 0).as_bool() {
        if msg.message != WM_PROBE {
            continue;
        }
        // the hook posts on every mouse move, only the newest position is worth a UIA roundtrip
        let mut newer = MSG::default();
        while PeekMessageW(&mut newer, None, WM_PROBE, WM_PROBE, PM_REMOVE).as_bool() {
            msg = newer;
        }
        let pt = POINT {
            x: msg.wParam.0 as isize as i32,
            y: msg.lParam.0 as i32,
        };
        let rect = find_volume_button(&automation, &audio, pt).ok().flatten();
        *VOLUME_RECT.lock().unwrap() = rect;
    }
}

unsafe fn find_volume_button(
    automation: &IUIAutomation,
    audio: &Audio,
    pt: POINT,
) -> Result<Option<RECT>> {
    let walker = automation.ControlViewWalker()?;
    let mut element = automation.ElementFromPoint(pt)?;

    // hit testing can land on a child of the button, so climb a few levels
    for _ in 0..4 {
        if element.CurrentAutomationId()?.to_string() == TRAY_BUTTON_ID {
            let name = element.CurrentName()?.to_string();
            if !is_volume_button(&name, audio) {
                return Ok(None);
            }
            return Ok(Some(element.CurrentBoundingRectangle()?));
        }
        match walker.GetParentElement(&element) {
            Ok(parent) => element = parent,
            Err(_) => break,
        }
    }
    Ok(None)
}

// hmm, the label is localized ("Volume" / "Birim" / ...) but it always embeds the
// output device name, e.g. "Volume Speakers (Realtek(R) Audio): 37%", so match on that
fn is_volume_button(name: &str, audio: &Audio) -> bool {
    match audio.device_name() {
        Ok(device) if !device.is_empty() => name.contains(&device),
        _ => name.starts_with("Volume"),
    }
}
