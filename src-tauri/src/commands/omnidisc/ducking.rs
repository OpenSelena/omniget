//! Keep Windows from muffling everything else the moment a call starts.
//!
//! Windows attenuates every other application by ~80 % while a process holds a
//! communications stream. Nobody asks for it and nothing in the app explains
//! it: the user joins a voice channel and the music, the game and the video
//! they deliberately left playing all drop. OmniDisc already has its own
//! ducking (`omnidisc.voice.ducking_percent`), it applies to the call mix, and
//! it is off unless the user turns it on — the system's version is a second,
//! invisible one on top.
//!
//! `IAudioSessionControl2::SetDuckingPreference(TRUE)` opts this process out.
//! It has to be set before the communications stream opens, which is why this
//! runs when the voice engine starts rather than when a call begins.
//!
//! No-op everywhere else: macOS and Linux do not duck other applications.

/// Ask Windows not to duck other applications for us. Best effort: failing
/// only means the platform default stays, so it never blocks a call.
pub fn opt_out_of_system_ducking() {
    #[cfg(windows)]
    {
        if std::thread::Builder::new()
            .name("omnidisc-ducking".into())
            .spawn(|| match windows_impl::apply() {
                Ok(()) => {
                    tracing::info!("[omnidisc] opted out of the Windows ducking of other apps")
                }
                Err(e) => tracing::warn!(
                    "[omnidisc] could not opt out of the Windows ducking of other apps: {}",
                    e
                ),
            })
            .is_err()
        {
            tracing::warn!("[omnidisc] could not start the ducking opt-out thread");
        }
    }
}

#[cfg(windows)]
mod windows_impl {
    use windows::core::Interface;
    use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
    use windows::Win32::Media::Audio::{
        eConsole, eRender, IAudioSessionControl2, IAudioSessionManager2, IMMDeviceEnumerator,
        MMDeviceEnumerator,
    };
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
    };

    pub fn apply() -> windows::core::Result<()> {
        unsafe {
            // Tauri's main thread is an STA; this runs on its own thread, so an
            // MTA is free here. `RPC_E_CHANGED_MODE` would only mean someone
            // else already initialised it — still usable, just not ours to undo.
            let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
            let owns_com = hr.is_ok();
            if hr.is_err() && hr != RPC_E_CHANGED_MODE {
                return Err(hr.into());
            }
            let result = set_preference();
            if owns_com {
                CoUninitialize();
            }
            result
        }
    }

    unsafe fn set_preference() -> windows::core::Result<()> {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
        let manager: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None)?;
        let session = manager.GetAudioSessionControl(None, 0)?;
        let session: IAudioSessionControl2 = session.cast()?;
        session.SetDuckingPreference(true)
    }
}
