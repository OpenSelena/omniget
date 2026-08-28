use super::resample::LinearResampler;
use livekit::webrtc::audio_frame::AudioFrame;
use livekit::webrtc::audio_source::native::NativeAudioSource;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const SAMPLE_RATE: u32 = 48_000;
pub const FRAME_SAMPLES: usize = 480;
const FRAME: Duration = Duration::from_millis(10);
const LEVEL_EVERY_TICKS: u32 = 10;
const VAD_HANGOVER_TICKS: u32 = 30;
const MAX_BACKLOG_MS: usize = 120;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CaptureEvent {
    Level { rms_db: f32, peak: f32 },
    Speaking(bool),
    Underrun,
}

pub struct CaptureFlags {
    pub muted: AtomicBool,
    pub ptt_enabled: AtomicBool,
    pub ptt_pressed: AtomicBool,
    pub denoise: AtomicBool,
    pub monitor: AtomicBool,
    vad_threshold_db: AtomicU32,
}

impl Default for CaptureFlags {
    fn default() -> Self {
        Self {
            muted: AtomicBool::new(false),
            ptt_enabled: AtomicBool::new(false),
            ptt_pressed: AtomicBool::new(false),
            denoise: AtomicBool::new(cfg!(feature = "rnnoise")),
            monitor: AtomicBool::new(false),
            vad_threshold_db: AtomicU32::new((-45.0f32).to_bits()),
        }
    }
}

impl CaptureFlags {
    pub fn vad_threshold_db(&self) -> f32 {
        f32::from_bits(self.vad_threshold_db.load(Ordering::Relaxed))
    }

    pub fn set_vad_threshold_db(&self, db: f32) {
        self.vad_threshold_db.store(db.to_bits(), Ordering::Relaxed);
    }

    pub fn transmitting(&self) -> bool {
        !self.muted.load(Ordering::Relaxed)
            && (!self.ptt_enabled.load(Ordering::Relaxed) || self.ptt_pressed.load(Ordering::Relaxed))
    }
}

pub enum FeederMsg {
    Input { consumer: rtrb::Consumer<f32>, sample_rate: u32 },
    NoInput,
    Source(Option<NativeAudioSource>),
    TestTone(Option<f32>),
    Stop,
}

pub type CaptureSink = Arc<dyn Fn(CaptureEvent) + Send + Sync>;

pub struct Feeder {
    tx: mpsc::Sender<FeederMsg>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Feeder {
    pub fn spawn(flags: Arc<CaptureFlags>, sink: CaptureSink) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("omnidisc-mic".into())
            .spawn(move || run(rx, flags, sink))
            .map_err(|e| format!("could not start the microphone thread: {e}"))?;
        Ok(Self { tx, thread: Some(thread) })
    }

    pub fn send(&self, msg: FeederMsg) {
        let _ = self.tx.send(msg);
    }
}

impl Drop for Feeder {
    fn drop(&mut self) {
        let _ = self.tx.send(FeederMsg::Stop);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

struct Pipeline {
    consumer: rtrb::Consumer<f32>,
    resampler: LinearResampler,
    sample_rate: u32,
    scratch: Vec<f32>,
    frame: Vec<f32>,
    out: AudioFrame<'static>,
    #[cfg(feature = "rnnoise")]
    denoiser: Box<nnnoiseless::DenoiseState<'static>>,
    #[cfg(feature = "rnnoise")]
    denoise_in: Vec<f32>,
    #[cfg(feature = "rnnoise")]
    denoise_out: Vec<f32>,
    speaking: bool,
    hang: u32,
    tick: u32,
    underruns: u64,
    tone: Option<(f32, f32)>,
}

impl Pipeline {
    fn new(consumer: rtrb::Consumer<f32>, sample_rate: u32) -> Self {
        Self {
            consumer,
            resampler: LinearResampler::new(sample_rate, SAMPLE_RATE),
            sample_rate,
            scratch: vec![0.0; 8192],
            frame: vec![0.0; FRAME_SAMPLES],
            out: AudioFrame::new(SAMPLE_RATE, 1, FRAME_SAMPLES as u32),
            #[cfg(feature = "rnnoise")]
            denoiser: nnnoiseless::DenoiseState::new(),
            #[cfg(feature = "rnnoise")]
            denoise_in: vec![0.0; FRAME_SAMPLES],
            #[cfg(feature = "rnnoise")]
            denoise_out: vec![0.0; FRAME_SAMPLES],
            speaking: false,
            hang: 0,
            tick: 0,
            underruns: 0,
            tone: None,
        }
    }

    fn pull(&mut self) {
        loop {
            let avail = self.consumer.slots();
            if avail == 0 {
                break;
            }
            let n = avail.min(self.scratch.len());
            let Ok(chunk) = self.consumer.read_chunk(n) else { break };
            let (a, b) = chunk.as_slices();
            self.scratch[..a.len()].copy_from_slice(a);
            self.scratch[a.len()..a.len() + b.len()].copy_from_slice(b);
            chunk.commit_all();
            self.resampler.push(&self.scratch[..n]);
            if n < self.scratch.len() {
                break;
            }
        }
        let max_backlog = (self.sample_rate as usize * MAX_BACKLOG_MS) / 1000;
        self.resampler.drop_excess(max_backlog);
    }

    fn step(&mut self, flags: &CaptureFlags, source: Option<&NativeAudioSource>, sink: &CaptureSink) {
        self.pull();
        let mut got = self.resampler.produce(&mut self.frame);
        if let Some((hz, phase)) = self.tone.as_mut() {
            let step = 2.0 * std::f32::consts::PI * *hz / SAMPLE_RATE as f32;
            for s in self.frame.iter_mut() {
                *s = phase.sin() * 0.5;
                *phase = (*phase + step) % (2.0 * std::f32::consts::PI);
            }
            got = true;
        }
        if !got {
            self.frame.fill(0.0);
            self.underruns += 1;
            if self.underruns == 50 {
                sink(CaptureEvent::Underrun);
            }
        } else {
            self.underruns = 0;
        }

        #[cfg(feature = "rnnoise")]
        if got && flags.denoise.load(Ordering::Relaxed) {
            for (d, s) in self.denoise_in.iter_mut().zip(self.frame.iter()) {
                *d = s * 32_768.0;
            }
            self.denoiser.process_frame(&mut self.denoise_out, &self.denoise_in);
            for (f, d) in self.frame.iter_mut().zip(self.denoise_out.iter()) {
                *f = (d / 32_768.0).clamp(-1.0, 1.0);
            }
        }

        let transmitting = flags.transmitting();
        let (rms, peak) = level(&self.frame);
        let rms_db = if rms > 0.0 { 20.0 * rms.log10() } else { -100.0 };

        let loud = got && transmitting && rms_db > flags.vad_threshold_db();
        if loud {
            self.hang = VAD_HANGOVER_TICKS;
        } else if self.hang > 0 {
            self.hang -= 1;
        }
        let speaking_now = self.hang > 0 && transmitting;
        if speaking_now != self.speaking {
            self.speaking = speaking_now;
            sink(CaptureEvent::Speaking(speaking_now));
        }

        self.tick = self.tick.wrapping_add(1);
        if flags.monitor.load(Ordering::Relaxed) && self.tick % LEVEL_EVERY_TICKS == 0 {
            sink(CaptureEvent::Level { rms_db, peak });
        }

        if let Some(src) = source {
            {
                let data = self.out.data.to_mut();
                if transmitting {
                    for (o, s) in data.iter_mut().zip(self.frame.iter()) {
                        *o = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                    }
                } else {
                    data.fill(0);
                }
            }
            if let Err(e) = futures::executor::block_on(src.capture_frame(&self.out)) {
                tracing::debug!("[omnidisc-media] capture_frame: {e}");
            }
        }
    }
}

fn level(frame: &[f32]) -> (f32, f32) {
    let mut sum = 0.0f32;
    let mut peak = 0.0f32;
    for s in frame {
        sum += s * s;
        peak = peak.max(s.abs());
    }
    ((sum / frame.len().max(1) as f32).sqrt(), peak)
}

fn run(rx: mpsc::Receiver<FeederMsg>, flags: Arc<CaptureFlags>, sink: CaptureSink) {
    let mut pipeline: Option<Pipeline> = None;
    let mut source: Option<NativeAudioSource> = None;
    let mut next = Instant::now();
    loop {
        let msg = if pipeline.is_some() {
            match rx.try_recv() {
                Ok(m) => Some(m),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => return,
            }
        } else {
            match rx.recv() {
                Ok(m) => Some(m),
                Err(_) => return,
            }
        };
        match msg {
            Some(FeederMsg::Stop) => return,
            Some(FeederMsg::Input { consumer, sample_rate }) => {
                pipeline = Some(Pipeline::new(consumer, sample_rate));
                next = Instant::now();
                continue;
            }
            Some(FeederMsg::NoInput) => {
                if pipeline.take().is_some() {
                    sink(CaptureEvent::Speaking(false));
                }
                continue;
            }
            Some(FeederMsg::Source(s)) => {
                source = s;
                continue;
            }
            Some(FeederMsg::TestTone(hz)) => {
                if pipeline.is_none() {
                    let (_keep, consumer) = rtrb::RingBuffer::<f32>::new(SAMPLE_RATE as usize);
                    std::mem::forget(_keep);
                    pipeline = Some(Pipeline::new(consumer, SAMPLE_RATE));
                    next = Instant::now();
                }
                if let Some(p) = pipeline.as_mut() {
                    p.tone = hz.map(|h| (h, 0.0));
                }
                continue;
            }
            None => {}
        }
        let Some(p) = pipeline.as_mut() else { continue };
        p.step(&flags, source.as_ref(), &sink);
        next += FRAME;
        let now = Instant::now();
        if next > now {
            std::thread::sleep(next - now);
        } else if now - next > Duration::from_millis(100) {
            next = now;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_of_silence_and_full_scale() {
        assert_eq!(level(&[0.0; 480]), (0.0, 0.0));
        let (rms, peak) = level(&[1.0; 480]);
        assert!((rms - 1.0).abs() < 1e-6);
        assert_eq!(peak, 1.0);
    }

    #[test]
    fn transmit_gate_respects_mute_and_ptt() {
        let f = CaptureFlags::default();
        assert!(f.transmitting());
        f.muted.store(true, Ordering::Relaxed);
        assert!(!f.transmitting());
        f.muted.store(false, Ordering::Relaxed);
        f.ptt_enabled.store(true, Ordering::Relaxed);
        assert!(!f.transmitting());
        f.ptt_pressed.store(true, Ordering::Relaxed);
        assert!(f.transmitting());
    }

    #[test]
    fn feeder_emits_speaking_on_loud_input_and_silence_when_gated() {
        let flags = Arc::new(CaptureFlags::default());
        let (etx, erx) = mpsc::channel();
        let sink: CaptureSink = Arc::new(move |e| {
            let _ = etx.send(e);
        });
        let feeder = Feeder::spawn(flags.clone(), sink).expect("feeder");
        let (mut prod, cons) = rtrb::RingBuffer::<f32>::new(48_000);
        feeder.send(FeederMsg::Input { consumer: cons, sample_rate: 48_000 });
        let mut phase = 0.0f32;
        for _ in 0..40 {
            let mut buf = [0.0f32; 480];
            for s in buf.iter_mut() {
                *s = (phase).sin() * 0.5;
                phase += 2.0 * std::f32::consts::PI * 440.0 / 48_000.0;
            }
            let _ = prod.push_partial_slice(&buf);
            std::thread::sleep(Duration::from_millis(10));
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut spoke = false;
        while Instant::now() < deadline {
            if let Ok(CaptureEvent::Speaking(true)) = erx.recv_timeout(Duration::from_millis(100)) {
                spoke = true;
                break;
            }
        }
        assert!(spoke, "loud sine never flagged as speaking");
        flags.muted.store(true, Ordering::Relaxed);
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut quiet = false;
        while Instant::now() < deadline {
            if let Ok(CaptureEvent::Speaking(false)) = erx.recv_timeout(Duration::from_millis(100)) {
                quiet = true;
                break;
            }
        }
        assert!(quiet, "mute never cleared the speaking flag");
        drop(feeder);
    }
}
