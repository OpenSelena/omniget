use super::capture::{Feeder, FeederMsg};
use super::devices::{self, DeviceKind};
use super::playback::{Mixer, OutputRenderer};
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

const INPUT_RING_SECONDS: usize = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioIoError {
    NoDevice,
    PermissionDenied,
    DeviceBusy,
    Unsupported(String),
    /// The stream handle died but the device is still there — WASAPI answers
    /// this when the default endpoint changes under a running stream.
    Invalidated(String),
    /// A glitch that did not stop anything: an over/underrun, or a route the
    /// backend already followed on its own.
    Transient(String),
    Other(String),
}

impl AudioIoError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::NoDevice => "no_device",
            Self::PermissionDenied => "permission_denied",
            Self::DeviceBusy => "device_busy",
            Self::Unsupported(_) => "unsupported",
            Self::Invalidated(_) => "invalidated",
            Self::Transient(_) => "transient",
            Self::Other(_) => "other",
        }
    }

    /// Did the stream actually stop? Tearing a call down and re-opening the
    /// device because one buffer arrived late is worse than the late buffer,
    /// and on WASAPI late buffers are routine under load.
    pub fn fatal(&self) -> bool {
        !matches!(self, Self::Transient(_))
    }
}

impl std::fmt::Display for AudioIoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoDevice => write!(f, "no audio device"),
            Self::PermissionDenied => write!(f, "audio permission denied"),
            Self::DeviceBusy => write!(f, "audio device busy"),
            Self::Unsupported(s) => write!(f, "unsupported audio config: {s}"),
            Self::Invalidated(s) => write!(f, "audio stream invalidated: {s}"),
            Self::Transient(s) => write!(f, "audio glitch: {s}"),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

/// Access-denied as it reaches us through `std::io::Error`'s message: the
/// Windows HRESULT `E_ACCESSDENIED` and the plain Win32 `ERROR_ACCESS_DENIED`.
/// Matching the number and not the sentence is what keeps this working on a
/// non-English Windows.
#[cfg_attr(not(windows), allow(dead_code))]
fn is_access_denied(message: &str) -> bool {
    message.contains("(os error -2147024891)") || message.contains("(os error 5)")
}

/// WASAPI answers a microphone blocked in the Windows privacy settings with a
/// bare `E_ACCESSDENIED`, which cpal cannot map and reports as a backend error.
/// Left unclassified it becomes "the mic failed" instead of "grant the
/// permission", which is the one message that would let the user fix it.
#[cfg(windows)]
fn platform_permission_denied(e: &cpal::Error) -> bool {
    e.message().map(is_access_denied).unwrap_or(false)
}

#[cfg(not(windows))]
fn platform_permission_denied(_: &cpal::Error) -> bool {
    false
}

fn classify(e: cpal::Error) -> AudioIoError {
    match e.kind() {
        cpal::ErrorKind::PermissionDenied => AudioIoError::PermissionDenied,
        cpal::ErrorKind::DeviceNotAvailable => AudioIoError::NoDevice,
        cpal::ErrorKind::DeviceBusy => AudioIoError::DeviceBusy,
        cpal::ErrorKind::UnsupportedConfig => AudioIoError::Unsupported(e.to_string()),
        cpal::ErrorKind::StreamInvalidated => AudioIoError::Invalidated(e.to_string()),
        cpal::ErrorKind::Xrun
        | cpal::ErrorKind::DeviceChanged
        | cpal::ErrorKind::RealtimeDenied => AudioIoError::Transient(e.to_string()),
        _ if platform_permission_denied(&e) => AudioIoError::PermissionDenied,
        _ => AudioIoError::Other(e.to_string()),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamFault {
    Input,
    Output,
}

/// Why a device stopped working. Told apart by re-opening it: the OS answers
/// "gone" and "not allowed" with different errors, and the two need different
/// help text — one is "plug it back in", the other is "grant the permission".
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceLoss {
    Unplugged,
    PermissionRevoked,
    Busy,
    Failed,
}

pub fn classify_loss(still_listed: bool, probe: &AudioIoError) -> DeviceLoss {
    match probe {
        AudioIoError::PermissionDenied => DeviceLoss::PermissionRevoked,
        AudioIoError::NoDevice => DeviceLoss::Unplugged,
        AudioIoError::DeviceBusy => DeviceLoss::Busy,
        _ if !still_listed => DeviceLoss::Unplugged,
        _ => DeviceLoss::Failed,
    }
}

pub type FaultSink = Arc<dyn Fn(StreamFault, AudioIoError) + Send + Sync>;

enum IoCmd {
    StartInput {
        device: Option<String>,
        reply: mpsc::Sender<Result<(), AudioIoError>>,
    },
    StopInput,
    StartOutput {
        device: Option<String>,
        reply: mpsc::Sender<Result<(), AudioIoError>>,
    },
    StopOutput,
    Shutdown,
}

pub struct AudioIo {
    tx: mpsc::Sender<IoCmd>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl AudioIo {
    pub fn spawn(
        feeder: Arc<Feeder>,
        mixer: Arc<Mixer>,
        faults: FaultSink,
    ) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("omnidisc-audio-io".into())
            .spawn(move || run(rx, feeder, mixer, faults))
            .map_err(|e| format!("could not start the audio thread: {e}"))?;
        Ok(Self {
            tx,
            thread: Some(thread),
        })
    }

    fn ask(
        &self,
        make: impl FnOnce(mpsc::Sender<Result<(), AudioIoError>>) -> IoCmd,
    ) -> Result<(), AudioIoError> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(make(reply_tx))
            .map_err(|_| AudioIoError::Other("audio thread is gone".into()))?;
        reply_rx
            .recv_timeout(Duration::from_secs(10))
            .map_err(|_| AudioIoError::Other("audio thread did not answer".into()))?
    }

    pub fn start_input(&self, device: Option<String>) -> Result<(), AudioIoError> {
        self.ask(|reply| IoCmd::StartInput { device, reply })
    }

    pub fn stop_input(&self) {
        let _ = self.tx.send(IoCmd::StopInput);
    }

    pub fn start_output(&self, device: Option<String>) -> Result<(), AudioIoError> {
        self.ask(|reply| IoCmd::StartOutput { device, reply })
    }

    pub fn stop_output(&self) {
        let _ = self.tx.send(IoCmd::StopOutput);
    }
}

impl Drop for AudioIo {
    fn drop(&mut self) {
        let _ = self.tx.send(IoCmd::Shutdown);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

fn run(rx: mpsc::Receiver<IoCmd>, feeder: Arc<Feeder>, mixer: Arc<Mixer>, faults: FaultSink) {
    let mut input: Option<cpal::Stream> = None;
    let mut output: Option<cpal::Stream> = None;
    while let Ok(cmd) = rx.recv() {
        match cmd {
            IoCmd::StartInput { device, reply } => {
                input = None;
                feeder.send(FeederMsg::NoInput);
                let res = build_input(device.as_deref(), &feeder, faults.clone());
                let _ = reply.send(res.map(|s| {
                    input = Some(s);
                }));
            }
            IoCmd::StopInput => {
                input = None;
                feeder.send(FeederMsg::NoInput);
            }
            IoCmd::StartOutput { device, reply } => {
                output = None;
                let res = build_output(device.as_deref(), mixer.clone(), faults.clone());
                let _ = reply.send(res.map(|s| {
                    output = Some(s);
                }));
            }
            IoCmd::StopOutput => {
                output = None;
            }
            IoCmd::Shutdown => break,
        }
    }
    drop(input);
    drop(output);
}

fn build_input(
    device_id: Option<&str>,
    feeder: &Feeder,
    faults: FaultSink,
) -> Result<cpal::Stream, AudioIoError> {
    super::permission::ensure_microphone_access()?;
    let device = devices::find(DeviceKind::Input, device_id).ok_or(AudioIoError::NoDevice)?;
    let supported = device.default_input_config().map_err(classify)?;
    let config: StreamConfig = supported.config();
    let channels = config.channels.max(1) as usize;
    let sample_rate = config.sample_rate;
    let (producer, consumer) =
        rtrb::RingBuffer::<f32>::new(sample_rate as usize * INPUT_RING_SECONDS);
    let err_cb = move |e: cpal::Error| {
        faults(StreamFault::Input, classify(e));
    };
    let stream = match supported.sample_format() {
        SampleFormat::F32 => device.build_input_stream(
            config,
            input_callback::<f32>(producer, channels, |s| s),
            err_cb,
            None,
        ),
        SampleFormat::I16 => device.build_input_stream(
            config,
            input_callback::<i16>(producer, channels, |s| s as f32 / 32_768.0),
            err_cb,
            None,
        ),
        SampleFormat::I32 => device.build_input_stream(
            config,
            input_callback::<i32>(producer, channels, |s| s as f32 / 2_147_483_648.0),
            err_cb,
            None,
        ),
        SampleFormat::U16 => device.build_input_stream(
            config,
            input_callback::<u16>(producer, channels, |s| (s as f32 - 32_768.0) / 32_768.0),
            err_cb,
            None,
        ),
        other => return Err(AudioIoError::Unsupported(format!("{other:?}"))),
    }
    .map_err(classify)?;
    stream.play().map_err(classify)?;
    feeder.send(FeederMsg::Input {
        consumer,
        sample_rate,
    });
    Ok(stream)
}

fn input_callback<T: Copy + Send + 'static>(
    mut producer: rtrb::Producer<f32>,
    channels: usize,
    convert: impl Fn(T) -> f32 + Send + 'static,
) -> impl FnMut(&[T], &cpal::InputCallbackInfo) + Send + 'static {
    // WASAPI does not hand over the same number of frames every callback, so a
    // fixed scratch buffer has to loop instead of dropping whatever did not
    // fit — a dropped tail is a click in the middle of a sentence.
    const CHUNK_FRAMES: usize = 2048;
    let mut mono: Vec<f32> = vec![0.0; CHUNK_FRAMES];
    move |data: &[T], _| {
        let inv = 1.0 / channels as f32;
        for block in data.chunks(CHUNK_FRAMES * channels) {
            let frames = block.len() / channels;
            for (o, frame) in mono[..frames]
                .iter_mut()
                .zip(block.chunks_exact(channels.max(1)))
            {
                let mut acc = 0.0f32;
                for s in frame {
                    acc += convert(*s);
                }
                *o = acc * inv;
            }
            let _ = producer.push_partial_slice(&mono[..frames]);
        }
    }
}

fn build_output(
    device_id: Option<&str>,
    mixer: Arc<Mixer>,
    faults: FaultSink,
) -> Result<cpal::Stream, AudioIoError> {
    let device = devices::find(DeviceKind::Output, device_id).ok_or(AudioIoError::NoDevice)?;
    let supported = device.default_output_config().map_err(classify)?;
    let config: StreamConfig = supported.config();
    let channels = config.channels.max(1);
    let sample_rate = config.sample_rate;
    let err_cb = move |e: cpal::Error| {
        faults(StreamFault::Output, classify(e));
    };
    let stream = match supported.sample_format() {
        SampleFormat::F32 => device.build_output_stream(
            config,
            output_callback::<f32>(mixer, sample_rate, channels, |s| s),
            err_cb,
            None,
        ),
        SampleFormat::I16 => device.build_output_stream(
            config,
            output_callback::<i16>(mixer, sample_rate, channels, |s| (s * 32_767.0) as i16),
            err_cb,
            None,
        ),
        SampleFormat::I32 => device.build_output_stream(
            config,
            output_callback::<i32>(mixer, sample_rate, channels, |s| {
                (s * 2_147_483_647.0) as i32
            }),
            err_cb,
            None,
        ),
        SampleFormat::U16 => device.build_output_stream(
            config,
            output_callback::<u16>(mixer, sample_rate, channels, |s| {
                ((s + 1.0) * 32_767.5) as u16
            }),
            err_cb,
            None,
        ),
        other => return Err(AudioIoError::Unsupported(format!("{other:?}"))),
    }
    .map_err(classify)?;
    stream.play().map_err(classify)?;
    Ok(stream)
}

fn output_callback<T: Copy + Send + 'static>(
    mixer: Arc<Mixer>,
    sample_rate: u32,
    channels: u16,
    convert: impl Fn(f32) -> T + Send + 'static,
) -> impl FnMut(&mut [T], &cpal::OutputCallbackInfo) + Send + 'static {
    let mut renderer = OutputRenderer::new(sample_rate, channels);
    let mut scratch: Vec<f32> = vec![0.0; 16_384];
    move |data: &mut [T], _| {
        // Growing once beats filling the tail with silence forever on a device
        // whose period is larger than the guess made here.
        if scratch.len() < data.len() {
            scratch.resize(data.len(), 0.0);
        }
        let n = data.len();
        renderer.render(&mixer, &mut scratch[..n]);
        for (d, s) in data.iter_mut().zip(scratch[..n].iter()) {
            *d = convert(*s);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_beats_everything_else() {
        assert_eq!(
            classify_loss(true, &AudioIoError::PermissionDenied),
            DeviceLoss::PermissionRevoked
        );
        assert_eq!(
            classify_loss(false, &AudioIoError::PermissionDenied),
            DeviceLoss::PermissionRevoked
        );
    }

    #[test]
    fn a_device_the_os_no_longer_lists_counts_as_unplugged() {
        assert_eq!(
            classify_loss(false, &AudioIoError::Other("gone".into())),
            DeviceLoss::Unplugged
        );
        assert_eq!(
            classify_loss(true, &AudioIoError::NoDevice),
            DeviceLoss::Unplugged
        );
    }

    #[test]
    fn a_glitch_does_not_end_the_stream_but_everything_else_does() {
        assert!(!AudioIoError::Transient("xrun".into()).fatal());
        assert!(AudioIoError::Invalidated("default changed".into()).fatal());
        assert!(AudioIoError::NoDevice.fatal());
        assert!(AudioIoError::PermissionDenied.fatal());
        assert!(AudioIoError::Other("boom".into()).fatal());
    }

    #[test]
    fn windows_access_denied_is_recognised_by_its_number() {
        assert!(is_access_denied("Access is denied. (os error -2147024891)"));
        assert!(is_access_denied("Acesso negado. (os error 5)"));
        assert!(!is_access_denied(
            "The device is not available. (os error -2147023728)"
        ));
        assert!(!is_access_denied("no error code here"));
    }

    #[test]
    fn cpal_kinds_map_to_the_right_severity() {
        use cpal::{Error, ErrorKind};
        assert_eq!(
            classify(Error::new(ErrorKind::Xrun)),
            AudioIoError::Transient(Error::new(ErrorKind::Xrun).to_string())
        );
        assert_eq!(
            classify(Error::new(ErrorKind::DeviceChanged)),
            AudioIoError::Transient(Error::new(ErrorKind::DeviceChanged).to_string())
        );
        assert_eq!(
            classify(Error::new(ErrorKind::StreamInvalidated)),
            AudioIoError::Invalidated(Error::new(ErrorKind::StreamInvalidated).to_string())
        );
        assert_eq!(
            classify(Error::new(ErrorKind::DeviceNotAvailable)),
            AudioIoError::NoDevice
        );
    }

    #[test]
    fn a_listed_device_that_will_not_open_is_a_plain_failure() {
        assert_eq!(
            classify_loss(true, &AudioIoError::Other("boom".into())),
            DeviceLoss::Failed
        );
        assert_eq!(
            classify_loss(true, &AudioIoError::DeviceBusy),
            DeviceLoss::Busy
        );
    }
}
