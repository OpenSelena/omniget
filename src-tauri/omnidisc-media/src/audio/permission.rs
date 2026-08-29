use super::io::AudioIoError;

/// CoreAudio never refuses a TCC-blocked microphone: AUHAL starts fine and
/// renders zeros forever, so cpal cannot classify the denial and the app looks
/// healthy while transmitting silence. Asking AVFoundation first is the only
/// way to get a real answer (and to raise the system prompt at all).
#[cfg(target_os = "macos")]
pub fn ensure_microphone_access() -> Result<(), AudioIoError> {
    use objc2::msg_send;
    use objc2::runtime::{AnyClass, Bool};
    use objc2_foundation::NSString;
    use std::sync::mpsc;
    use std::time::Duration;

    let Some(cls) = AnyClass::get(c"AVCaptureDevice") else {
        return Ok(());
    };
    let media = NSString::from_str("soun");
    let status: isize = unsafe { msg_send![cls, authorizationStatusForMediaType: &*media] };
    match status {
        3 => Ok(()),
        1 | 2 => Err(AudioIoError::PermissionDenied),
        _ => {
            let (tx, rx) = mpsc::channel();
            let handler = block2::RcBlock::new(move |granted: Bool| {
                let _ = tx.send(granted.as_bool());
            });
            let _: () = unsafe {
                msg_send![cls, requestAccessForMediaType: &*media, completionHandler: &*handler]
            };
            match rx.recv_timeout(Duration::from_secs(8)) {
                Ok(true) => Ok(()),
                _ => Err(AudioIoError::PermissionDenied),
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn ensure_microphone_access() -> Result<(), AudioIoError> {
    Ok(())
}
