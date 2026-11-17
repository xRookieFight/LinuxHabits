use windows::core::{Result, BSTR};
use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Foundation::BOOL;
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{
    eMultimedia, eRender, IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator,
};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL, STGM_READ};

#[derive(Clone, Copy, Default)]
pub struct VolumeState {
    pub percent: u32,
    pub muted: bool,
}

/// Talks to whatever the default playback device is right now. COM must already
/// be initialized on the calling thread.
pub struct Audio {
    enumerator: IMMDeviceEnumerator,
}

impl Audio {
    pub fn new() -> Result<Self> {
        let enumerator = unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)? };
        Ok(Self { enumerator })
    }

    // looked up on every call on purpose, the user can switch outputs at any time
    fn default_device(&self) -> Result<IMMDevice> {
        unsafe { self.enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia) }
    }

    fn endpoint_volume(&self) -> Result<IAudioEndpointVolume> {
        unsafe { self.default_device()?.Activate(CLSCTX_ALL, None) }
    }

    /// Friendly name of the default output, e.g. "Speakers (Realtek(R) Audio)".
    pub fn device_name(&self) -> Result<String> {
        unsafe {
            let store = self.default_device()?.OpenPropertyStore(STGM_READ)?;
            let value = store.GetValue(&PKEY_Device_FriendlyName)?;
            Ok(BSTR::try_from(&value)?.to_string())
        }
    }

    /// Moves master volume by `delta` (scalar, so 0.02 is 2%) and returns where it ended up.
    /// Going up also unmutes, same as most Linux mixers do.
    pub fn step(&self, delta: f32) -> Result<VolumeState> {
        unsafe {
            let endpoint = self.endpoint_volume()?;
            let current = endpoint.GetMasterVolumeLevelScalar()?;
            // snap to whole percents, otherwise float drift shows up as 39 instead of 40
            let level = ((current + delta).clamp(0.0, 1.0) * 100.0).round() / 100.0;
            endpoint.SetMasterVolumeLevelScalar(level, std::ptr::null())?;

            let mut muted = endpoint.GetMute()?.as_bool();
            if muted && delta > 0.0 {
                endpoint.SetMute(BOOL::from(false), std::ptr::null())?;
                muted = false;
            }

            Ok(VolumeState {
                percent: (level * 100.0).round() as u32,
                muted,
            })
        }
    }

    /// Flips mute on the default output and returns the new state.
    pub fn toggle_mute(&self) -> Result<VolumeState> {
        unsafe {
            let endpoint = self.endpoint_volume()?;
            let muted = !endpoint.GetMute()?.as_bool();
            endpoint.SetMute(BOOL::from(muted), std::ptr::null())?;
            let level = endpoint.GetMasterVolumeLevelScalar()?;
            Ok(VolumeState {
                percent: (level * 100.0).round() as u32,
                muted,
            })
        }
    }
}
