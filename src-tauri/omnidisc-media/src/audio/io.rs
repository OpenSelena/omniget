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
    Other(String),
}

impl AudioIoError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::NoDevice => "no_device",
            Self::PermissionDenied => "permission_denied",
            Self::DeviceBusy => "device_busy",
            Self::Unsupported(_) => "unsupported",
            Self::Other(_) => "other",
        }
    }
}

impl std::fmt::Display for AudioIoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoDevice => write!(f, "no audio device"),
            Self::PermissionDenied => write!(f, "audio permission denied"),
            Self::DeviceBusy => write!(f, "audio device busy"),
            Self::Unsupported(s) => write!(f, "unsupported audio config: {s}"),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

fn classify(e: cpal::Error) -> AudioIoError {
    match e.kind() {
        cpal::ErrorKind::PermissionDenied => AudioIoError::PermissionDenied,
        cpal::ErrorKind::DeviceNotAvailable => AudioIoError::NoDevice,
        cpal::ErrorKind::DeviceBusy => AudioIoError::DeviceBusy,
        cpal::ErrorKind::UnsupportedConfig => AudioIoError::Unsupported(e.to_string()),
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

pub type FaultSink = Arc<dyn Fn(StreamFault, String) + Send + Sync>;

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
    let device = devices::find(DeviceKind::Input, device_id).ok_or(AudioIoError::NoDevice)?;
    let supported = device.default_input_config().map_err(classify)?;
    let config: StreamConfig = supported.config();
    let channels = config.channels.max(1) as usize;
    let sample_rate = config.sample_rate;
    let (producer, consumer) =
        rtrb::RingBuffer::<f32>::new(sample_rate as usize * INPUT_RING_SECONDS);
    let err_cb = move |e: cpal::Error| {
        faults(StreamFault::Input, e.to_string());
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
    let mut mono: Vec<f32> = Vec::with_capacity(8192);
    move |data: &[T], _| {
        mono.clear();
        let inv = 1.0 / channels as f32;
        for frame in data.chunks(channels) {
            let mut acc = 0.0f32;
            for s in frame {
                acc += convert(*s);
            }
            if mono.len() < mono.capacity() {
                mono.push(acc * inv);
            }
        }
        let _ = producer.push_partial_slice(&mono);
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
        faults(StreamFault::Output, e.to_string());
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
        let n = data.len().min(scratch.len());
        renderer.render(&mixer, &mut scratch[..n]);
        for (d, s) in data.iter_mut().zip(scratch[..n].iter()) {
            *d = convert(*s);
        }
        for d in data.iter_mut().skip(n) {
            *d = convert(0.0);
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
