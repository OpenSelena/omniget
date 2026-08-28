//! Goertzel meter over the mixed remote audio.
//!
//! The voice e2e tests used to assert "B received N non-silent frames", which a
//! stream of pure garbage also satisfies: when a frame fails to decrypt, Opus
//! packet-loss concealment invents comfort noise, so amplitude alone cannot tell
//! "the tone arrived" from "the decryptor rejected everything". This measures
//! how much of the received energy actually sits at the published tone, which is
//! what the tests mean to prove in both directions.

const BLOCK: usize = 1200;

#[derive(Debug)]
pub struct ToneMeter {
    coeff: f64,
    q1: f64,
    q2: f64,
    filled: usize,
    tone_energy: f64,
    total_energy: f64,
}

impl ToneMeter {
    /// `hz` must divide the sample rate into `BLOCK` evenly to land on a bin
    /// centre; 440 Hz at 48 kHz gives k = 11 exactly.
    pub fn new(hz: f64, sample_rate: f64) -> Self {
        let k = (BLOCK as f64 * hz / sample_rate).round();
        Self {
            coeff: 2.0 * (2.0 * std::f64::consts::PI * k / BLOCK as f64).cos(),
            q1: 0.0,
            q2: 0.0,
            filled: 0,
            tone_energy: 0.0,
            total_energy: 0.0,
        }
    }

    pub fn push(&mut self, samples: &[f32]) {
        for s in samples {
            let x = *s as f64;
            self.total_energy += x * x;
            let q0 = self.coeff * self.q1 - self.q2 + x;
            self.q2 = std::mem::replace(&mut self.q1, q0);
            self.filled += 1;
            if self.filled == BLOCK {
                let magnitude =
                    self.q1 * self.q1 + self.q2 * self.q2 - self.coeff * self.q1 * self.q2;
                self.tone_energy += magnitude / (BLOCK as f64 / 2.0);
                self.q1 = 0.0;
                self.q2 = 0.0;
                self.filled = 0;
            }
        }
    }

    /// Share of the received energy sitting at the measured tone, 0.0 when
    /// nothing arrived. A clean 440 Hz sine lands near 1.0; comfort noise and
    /// undecryptable frames stay near 0.
    pub fn ratio(&self) -> f64 {
        if self.total_energy <= f64::EPSILON {
            return 0.0;
        }
        (self.tone_energy / self.total_energy).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(hz: f64, samples: usize) -> Vec<f32> {
        (0..samples)
            .map(|i| (2.0 * std::f64::consts::PI * hz * i as f64 / 48_000.0).sin() as f32)
            .collect()
    }

    #[test]
    fn a_pure_tone_dominates_the_ratio() {
        let mut m = ToneMeter::new(440.0, 48_000.0);
        m.push(&sine(440.0, BLOCK * 8));
        assert!(m.ratio() > 0.9, "ratio was {}", m.ratio());
    }

    #[test]
    fn another_tone_does_not_register() {
        let mut m = ToneMeter::new(440.0, 48_000.0);
        m.push(&sine(1_000.0, BLOCK * 8));
        assert!(m.ratio() < 0.1, "ratio was {}", m.ratio());
    }

    #[test]
    fn noise_does_not_register() {
        let mut m = ToneMeter::new(440.0, 48_000.0);
        let mut seed = 0x9e3779b97f4a7c15u64;
        let noise: Vec<f32> = (0..BLOCK * 8)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                (seed as i32 as f64 / i32::MAX as f64) as f32
            })
            .collect();
        m.push(&noise);
        assert!(m.ratio() < 0.1, "ratio was {}", m.ratio());
    }

    #[test]
    fn silence_reports_nothing() {
        let mut m = ToneMeter::new(440.0, 48_000.0);
        m.push(&vec![0.0; BLOCK * 2]);
        assert_eq!(m.ratio(), 0.0);
    }
}
