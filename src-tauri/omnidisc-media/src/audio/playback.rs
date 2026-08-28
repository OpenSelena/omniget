use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Mutex;

pub const MIX_RATE: u32 = 48_000;
const MAX_SOURCES: usize = 64;
const SCRATCH: usize = 16_384;
/// Ducking ramps instead of stepping: a hard gain jump on a voice track is an
/// audible click, and the click is worse than the thing ducking fixes.
const DUCK_RAMP_SECONDS: f32 = 0.08;

pub struct MixSource {
    pub key: String,
    pub user_id: String,
    pub consumer: rtrb::Consumer<f32>,
    pub gain: f32,
}

pub struct Mixer {
    sources: Mutex<Vec<MixSource>>,
    master: AtomicU32,
    deafened: AtomicBool,
    duck_amount: AtomicU32,
    ducking: AtomicBool,
    duck_gain: AtomicU32,
    pub underruns: AtomicU64,
    pub lock_misses: AtomicU64,
}

impl Default for Mixer {
    fn default() -> Self {
        Self {
            sources: Mutex::new(Vec::with_capacity(MAX_SOURCES)),
            master: AtomicU32::new(1.0f32.to_bits()),
            deafened: AtomicBool::new(false),
            duck_amount: AtomicU32::new(0.0f32.to_bits()),
            ducking: AtomicBool::new(false),
            duck_gain: AtomicU32::new(1.0f32.to_bits()),
            underruns: AtomicU64::new(0),
            lock_misses: AtomicU64::new(0),
        }
    }
}

impl Mixer {
    pub fn add_source(&self, key: String, user_id: String, consumer: rtrb::Consumer<f32>, gain: f32) {
        if let Ok(mut s) = self.sources.lock() {
            s.retain(|x| x.key != key);
            if s.len() < MAX_SOURCES {
                s.push(MixSource { key, user_id, consumer, gain: gain.clamp(0.0, 2.0) });
            }
        }
    }

    pub fn remove_source(&self, key: &str) {
        if let Ok(mut s) = self.sources.lock() {
            s.retain(|x| x.key != key);
        }
    }

    pub fn clear(&self) {
        if let Ok(mut s) = self.sources.lock() {
            s.clear();
        }
    }

    pub fn set_user_gain(&self, user_id: &str, gain: f32) {
        if let Ok(mut s) = self.sources.lock() {
            for x in s.iter_mut().filter(|x| x.user_id == user_id) {
                x.gain = gain.clamp(0.0, 2.0);
            }
        }
    }

    pub fn set_master(&self, gain: f32) {
        self.master.store(gain.clamp(0.0, 2.0).to_bits(), Ordering::Relaxed);
    }

    pub fn master(&self) -> f32 {
        f32::from_bits(self.master.load(Ordering::Relaxed))
    }

    pub fn set_deafened(&self, on: bool) {
        self.deafened.store(on, Ordering::Relaxed);
    }

    pub fn is_deafened(&self) -> bool {
        self.deafened.load(Ordering::Relaxed)
    }

    /// How much of everyone else to take away while the local user speaks,
    /// 0–100 %. 0 turns ducking off.
    pub fn set_duck_percent(&self, percent: u8) {
        let amount = (percent.min(100) as f32) / 100.0;
        self.duck_amount.store(amount.to_bits(), Ordering::Relaxed);
    }

    pub fn duck_percent(&self) -> u8 {
        (f32::from_bits(self.duck_amount.load(Ordering::Relaxed)) * 100.0).round() as u8
    }

    pub fn set_ducking(&self, on: bool) {
        self.ducking.store(on, Ordering::Relaxed);
    }

    fn duck_target(&self) -> f32 {
        if self.ducking.load(Ordering::Relaxed) {
            1.0 - f32::from_bits(self.duck_amount.load(Ordering::Relaxed)).clamp(0.0, 1.0)
        } else {
            1.0
        }
    }

    pub fn source_count(&self) -> usize {
        self.sources.lock().map(|s| s.len()).unwrap_or(0)
    }

    pub fn mix_into(&self, out: &mut [f32], scratch: &mut [f32]) {
        out.fill(0.0);
        if self.is_deafened() {
            return;
        }
        let Ok(mut sources) = self.sources.try_lock() else {
            self.lock_misses.fetch_add(1, Ordering::Relaxed);
            return;
        };
        let master = self.master();
        let n = out.len().min(scratch.len());
        for src in sources.iter_mut() {
            let avail = src.consumer.slots().min(n);
            if avail < n {
                self.underruns.fetch_add(1, Ordering::Relaxed);
            }
            if avail == 0 {
                continue;
            }
            let Ok(chunk) = src.consumer.read_chunk(avail) else { continue };
            let (a, b) = chunk.as_slices();
            scratch[..a.len()].copy_from_slice(a);
            scratch[a.len()..a.len() + b.len()].copy_from_slice(b);
            chunk.commit_all();
            let g = src.gain * master;
            for (o, s) in out[..avail].iter_mut().zip(scratch[..avail].iter()) {
                *o += s * g;
            }
        }
        drop(sources);
        let target = self.duck_target();
        let mut g = f32::from_bits(self.duck_gain.load(Ordering::Relaxed));
        let step = 1.0 / (MIX_RATE as f32 * DUCK_RAMP_SECONDS);
        for o in out.iter_mut() {
            if g < target {
                g = (g + step).min(target);
            } else if g > target {
                g = (g - step).max(target);
            }
            *o = (*o * g).clamp(-1.0, 1.0);
        }
        self.duck_gain.store(g.to_bits(), Ordering::Relaxed);
    }
}

pub struct OutputRenderer {
    step: f64,
    pos: f64,
    channels: usize,
    buf48: VecDeque<f32>,
    mix: Vec<f32>,
    scratch: Vec<f32>,
}

impl OutputRenderer {
    pub fn new(out_rate: u32, channels: u16) -> Self {
        Self {
            step: MIX_RATE as f64 / out_rate.max(1) as f64,
            pos: 0.0,
            channels: channels.max(1) as usize,
            buf48: VecDeque::with_capacity(SCRATCH),
            mix: vec![0.0; SCRATCH],
            scratch: vec![0.0; SCRATCH],
        }
    }

    pub fn render(&mut self, mixer: &Mixer, data: &mut [f32]) {
        let frames = data.len() / self.channels;
        if frames == 0 {
            return;
        }
        if (self.step - 1.0).abs() < f64::EPSILON {
            let n = frames.min(self.mix.len());
            let (mix, scratch) = (&mut self.mix[..n], &mut self.scratch[..n]);
            mixer.mix_into(mix, scratch);
            for (i, frame) in data.chunks_mut(self.channels).enumerate() {
                let s = if i < n { mix[i] } else { 0.0 };
                frame.fill(s);
            }
            return;
        }
        let needed = (self.pos + frames as f64 * self.step).floor() as usize + 2;
        if self.buf48.len() < needed {
            let n = (needed - self.buf48.len()).min(self.mix.len());
            let (mix, scratch) = (&mut self.mix[..n], &mut self.scratch[..n]);
            mixer.mix_into(mix, scratch);
            for s in mix.iter() {
                if self.buf48.len() < self.buf48.capacity() {
                    self.buf48.push_back(*s);
                }
            }
        }
        let mut p = self.pos;
        for frame in data.chunks_mut(self.channels) {
            let idx = p.floor() as usize;
            let frac = (p - idx as f64) as f32;
            let a = self.buf48.get(idx).copied().unwrap_or(0.0);
            let b = self.buf48.get(idx + 1).copied().unwrap_or(a);
            frame.fill(a + (b - a) * frac);
            p += self.step;
        }
        let consumed = (p.floor() as usize).min(self.buf48.len());
        self.pos = p - p.floor();
        self.buf48.drain(..consumed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixes_two_sources_with_gain_and_master() {
        let mixer = Mixer::default();
        let (mut p1, c1) = rtrb::RingBuffer::<f32>::new(1024);
        let (mut p2, c2) = rtrb::RingBuffer::<f32>::new(1024);
        mixer.add_source("t1".into(), "u1".into(), c1, 1.0);
        mixer.add_source("t2".into(), "u2".into(), c2, 0.5);
        let _ = p1.push_partial_slice(&[0.2; 480]);
        let _ = p2.push_partial_slice(&[0.4; 480]);
        let mut out = vec![0.0; 480];
        let mut scratch = vec![0.0; 480];
        mixer.mix_into(&mut out, &mut scratch);
        assert!((out[0] - 0.4).abs() < 1e-6);
        mixer.set_user_gain("u2", 2.0);
        mixer.set_master(0.5);
        let _ = p1.push_partial_slice(&[0.2; 480]);
        let _ = p2.push_partial_slice(&[0.4; 480]);
        mixer.mix_into(&mut out, &mut scratch);
        assert!((out[0] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn deafen_outputs_silence_and_underrun_fills_zero() {
        let mixer = Mixer::default();
        let (mut p1, c1) = rtrb::RingBuffer::<f32>::new(1024);
        mixer.add_source("t1".into(), "u1".into(), c1, 1.0);
        let _ = p1.push_partial_slice(&[0.9; 100]);
        let mut out = vec![1.0; 480];
        let mut scratch = vec![0.0; 480];
        mixer.mix_into(&mut out, &mut scratch);
        assert!((out[0] - 0.9).abs() < 1e-6);
        assert_eq!(out[200], 0.0);
        mixer.set_deafened(true);
        let _ = p1.push_partial_slice(&[0.9; 100]);
        mixer.mix_into(&mut out, &mut scratch);
        assert!(out.iter().all(|s| *s == 0.0));
    }

    #[test]
    fn ducking_ramps_other_people_down_and_back_up() {
        let mixer = Mixer::default();
        let (mut p1, c1) = rtrb::RingBuffer::<f32>::new(48_000);
        mixer.add_source("t1".into(), "u1".into(), c1, 1.0);
        mixer.set_duck_percent(60);
        assert_eq!(mixer.duck_percent(), 60);
        let mut out = vec![0.0; 4800];
        let mut scratch = vec![0.0; 4800];

        let _ = p1.push_partial_slice(&vec![0.5; 4800]);
        mixer.mix_into(&mut out, &mut scratch);
        assert!((out[4799] - 0.5).abs() < 1e-6, "no ducking while nobody speaks");

        mixer.set_ducking(true);
        for _ in 0..2 {
            let _ = p1.push_partial_slice(&vec![0.5; 4800]);
            mixer.mix_into(&mut out, &mut scratch);
        }
        assert!((out[4799] - 0.2).abs() < 1e-3, "ducked sample was {}", out[4799]);

        mixer.set_ducking(false);
        for _ in 0..2 {
            let _ = p1.push_partial_slice(&vec![0.5; 4800]);
            mixer.mix_into(&mut out, &mut scratch);
        }
        assert!((out[4799] - 0.5).abs() < 1e-3, "restored sample was {}", out[4799]);
    }

    #[test]
    fn ducking_off_by_default_leaves_the_mix_alone() {
        let mixer = Mixer::default();
        let (mut p1, c1) = rtrb::RingBuffer::<f32>::new(4096);
        mixer.add_source("t1".into(), "u1".into(), c1, 1.0);
        mixer.set_ducking(true);
        let _ = p1.push_partial_slice(&[0.5; 480]);
        let mut out = vec![0.0; 480];
        let mut scratch = vec![0.0; 480];
        mixer.mix_into(&mut out, &mut scratch);
        assert!((out[479] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn renderer_resamples_to_device_rate_and_channels() {
        let mixer = Mixer::default();
        let (mut p1, c1) = rtrb::RingBuffer::<f32>::new(48_000);
        mixer.add_source("t1".into(), "u1".into(), c1, 1.0);
        let _ = p1.push_partial_slice(&vec![0.5; 4800]);
        let mut r = OutputRenderer::new(44_100, 2);
        let mut data = vec![0.0; 441 * 2];
        r.render(&mixer, &mut data);
        assert!((data[0] - 0.5).abs() < 1e-6);
        assert!((data[1] - 0.5).abs() < 1e-6);
        assert!((data[880] - 0.5).abs() < 1e-6);
    }
}
