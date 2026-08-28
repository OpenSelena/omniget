pub struct LinearResampler {
    step: f64,
    pos: f64,
    pending: Vec<f32>,
}

impl LinearResampler {
    pub fn new(from_rate: u32, to_rate: u32) -> Self {
        let step = if to_rate == 0 { 1.0 } else { from_rate as f64 / to_rate as f64 };
        Self { step, pos: 0.0, pending: Vec::with_capacity(16_384) }
    }

    pub fn is_identity(&self) -> bool {
        (self.step - 1.0).abs() < f64::EPSILON
    }

    pub fn push(&mut self, input: &[f32]) {
        self.pending.extend_from_slice(input);
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    pub fn needed_for(&self, out_len: usize) -> usize {
        (self.pos + out_len as f64 * self.step).floor() as usize + 2
    }

    pub fn drop_excess(&mut self, keep_at_most: usize) {
        if self.pending.len() > keep_at_most {
            let drop = self.pending.len() - keep_at_most;
            self.pending.drain(..drop);
        }
    }

    pub fn produce(&mut self, out: &mut [f32]) -> bool {
        if self.is_identity() {
            if self.pending.len() < out.len() {
                return false;
            }
            out.copy_from_slice(&self.pending[..out.len()]);
            self.pending.drain(..out.len());
            return true;
        }
        if self.pending.len() < self.needed_for(out.len()) {
            return false;
        }
        let mut p = self.pos;
        for o in out.iter_mut() {
            let idx = p.floor() as usize;
            let frac = (p - idx as f64) as f32;
            let a = self.pending[idx];
            let b = self.pending[idx + 1];
            *o = a + (b - a) * frac;
            p += self.step;
        }
        let consumed = p.floor() as usize;
        self.pos = p - consumed as f64;
        self.pending.drain(..consumed);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_passes_through() {
        let mut r = LinearResampler::new(48_000, 48_000);
        r.push(&[1.0, 2.0, 3.0, 4.0]);
        let mut out = [0.0; 4];
        assert!(r.produce(&mut out));
        assert_eq!(out, [1.0, 2.0, 3.0, 4.0]);
        assert_eq!(r.pending_len(), 0);
    }

    #[test]
    fn upsamples_ramp_smoothly() {
        let mut r = LinearResampler::new(24_000, 48_000);
        let input: Vec<f32> = (0..100).map(|i| i as f32).collect();
        r.push(&input);
        let mut out = vec![0.0; 100];
        assert!(r.produce(&mut out));
        for (i, v) in out.iter().enumerate() {
            assert!((v - i as f32 * 0.5).abs() < 1e-4, "index {i}: {v}");
        }
    }

    #[test]
    fn reports_underrun_without_consuming() {
        let mut r = LinearResampler::new(44_100, 48_000);
        r.push(&[0.0; 10]);
        let mut out = [0.0; 480];
        assert!(!r.produce(&mut out));
        assert_eq!(r.pending_len(), 10);
    }

    #[test]
    fn downsample_keeps_fractional_position_across_calls() {
        let mut r = LinearResampler::new(44_100, 48_000);
        let input: Vec<f32> = (0..2000).map(|i| (i as f32 * 0.01).sin()).collect();
        r.push(&input);
        let mut a = vec![0.0; 480];
        let mut b = vec![0.0; 480];
        assert!(r.produce(&mut a));
        assert!(r.produce(&mut b));
        let expected = ((480.0 * 44_100.0 / 48_000.0) as f32 * 0.01).sin();
        assert!((b[0] - expected).abs() < 0.01);
    }
}
