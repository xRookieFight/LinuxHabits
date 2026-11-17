use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use windows::core::w;
use windows::Win32::UI::WindowsAndMessaging::RegisterWindowMessageW;

// read from the mouse hook on every event, so plain atomics instead of a locked struct
static VOLUME_SCROLL: AtomicBool = AtomicBool::new(true);
static VOLUME_MIDDLE_CLICK_MUTE: AtomicBool = AtomicBool::new(true);

pub fn volume_scroll() -> bool {
    VOLUME_SCROLL.load(Ordering::Relaxed)
}

pub fn volume_middle_click_mute() -> bool {
    VOLUME_MIDDLE_CLICK_MUTE.load(Ordering::Relaxed)
}

/// %APPDATA%\LinuxHabits\config.toml, shared with the settings app.
pub fn path() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|dir| PathBuf::from(dir).join("LinuxHabits").join("config.toml"))
}

/// (Re)reads the config file. Missing file or unknown keys just leave the defaults,
/// every feature is on until someone turns it off.
pub fn load() {
    VOLUME_SCROLL.store(true, Ordering::Relaxed);
    VOLUME_MIDDLE_CLICK_MUTE.store(true, Ordering::Relaxed);

    let Some(text) = path().and_then(|p| std::fs::read_to_string(p).ok()) else {
        return;
    };
    // hmm, only flat `key = true/false` lines exist so far, not worth pulling in a toml crate yet
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else { continue };
        let enabled = match value.trim() {
            "true" => true,
            "false" => false,
            _ => continue,
        };
        match key.trim() {
            "volume_scroll" => VOLUME_SCROLL.store(enabled, Ordering::Relaxed),
            "volume_middle_click_mute" => VOLUME_MIDDLE_CLICK_MUTE.store(enabled, Ordering::Relaxed),
            _ => {}
        }
    }
}

/// Sent by the settings app after it saves the config.
pub fn reload_message() -> u32 {
    static MSG: OnceLock<u32> = OnceLock::new();
    *MSG.get_or_init(|| unsafe { RegisterWindowMessageW(w!("LinuxHabits.ReloadConfig")) })
}

/// Sent by the settings app to shut the daemon down cleanly.
pub fn quit_message() -> u32 {
    static MSG: OnceLock<u32> = OnceLock::new();
    *MSG.get_or_init(|| unsafe { RegisterWindowMessageW(w!("LinuxHabits.Quit")) })
}
