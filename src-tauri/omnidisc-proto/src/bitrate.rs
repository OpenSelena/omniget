use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const RESOLUTIONS: [u16; 5] = [540, 720, 1080, 1440, 2160];
pub const FRAMERATES: [u16; 6] = [15, 30, 60, 90, 120, 144];

pub fn width_for_height(height: u16) -> u16 {
    match height {
        540 => 960,
        720 => 1280,
        1080 => 1920,
        1440 => 2560,
        2160 => 3840,
        h => ((h as u32 * 16 / 9) & !1) as u16,
    }
}

pub fn default_kbps(height: u16, fps: u16) -> u32 {
    let base = match height {
        540 => 1_500u32,
        720 => 2_500,
        1080 => 5_000,
        1440 => 9_000,
        2160 => 16_000,
        _ => {
            let px = width_for_height(height) as u64 * height as u64;
            ((px as f64 / (1920.0 * 1080.0)) * 5_000.0) as u32
        }
    };
    let scale = match fps {
        15 => 0.65,
        30 => 1.0,
        60 => 1.6,
        90 => 2.1,
        120 => 2.6,
        144 => 2.9,
        f => (f as f64 / 30.0).powf(0.7),
    };
    ((base as f64 * scale) as u32).max(300)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamingPolicy {
    pub max_height: u16,
    pub max_fps: u16,
    pub min_kbps: u32,
    pub max_kbps: u32,
    pub step_kbps: u32,
    pub allow_custom_bitrate: bool,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub overrides: BTreeMap<String, u32>,
    #[serde(default)]
    pub preferred_codec: Codec,
    #[serde(default = "default_true")]
    pub allow_h265: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Codec {
    #[default]
    H264,
    H265,
    Vp8,
    Vp9,
    Av1,
}

impl Default for StreamingPolicy {
    fn default() -> Self {
        Self {
            max_height: 2160,
            max_fps: 120,
            min_kbps: 500,
            max_kbps: 20_000,
            step_kbps: 500,
            allow_custom_bitrate: true,
            overrides: BTreeMap::new(),
            preferred_codec: Codec::H264,
            allow_h265: true,
        }
    }
}

impl StreamingPolicy {
    pub fn kbps_for(&self, height: u16, fps: u16) -> u32 {
        let key = format!("{height}p{fps}");
        let raw = self
            .overrides
            .get(&key)
            .copied()
            .unwrap_or_else(|| default_kbps(height, fps));
        raw.clamp(self.min_kbps, self.max_kbps)
    }

    pub fn clamp_custom(&self, requested_kbps: u32) -> u32 {
        if !self.allow_custom_bitrate {
            return self.max_kbps.min(requested_kbps).max(self.min_kbps);
        }
        let stepped = (requested_kbps / self.step_kbps.max(1)) * self.step_kbps.max(1);
        stepped.clamp(self.min_kbps, self.max_kbps)
    }

    pub fn allowed_resolutions(&self) -> Vec<u16> {
        RESOLUTIONS
            .iter()
            .copied()
            .filter(|h| *h <= self.max_height)
            .collect()
    }

    pub fn allowed_framerates(&self) -> Vec<u16> {
        FRAMERATES
            .iter()
            .copied()
            .filter(|f| *f <= self.max_fps)
            .collect()
    }

    pub fn codec_for(&self, height: u16, fps: u16) -> Codec {
        if self.allow_h265 && height >= 2160 && fps > 60 {
            Codec::H265
        } else {
            self.preferred_codec
        }
    }

    pub fn native_kbps(&self, width: u32, height: u32, fps: u16) -> u32 {
        let px = width as u64 * height as u64;
        let nearest = RESOLUTIONS
            .iter()
            .copied()
            .min_by_key(|h| {
                let hp = width_for_height(*h) as u64 * *h as u64;
                hp.abs_diff(px)
            })
            .unwrap_or(1080);
        let nearest_fps = FRAMERATES
            .iter()
            .copied()
            .min_by_key(|f| f.abs_diff(fps))
            .unwrap_or(30);
        let base = self.kbps_for(nearest, nearest_fps) as f64;
        let nearest_px = (width_for_height(nearest) as u64 * nearest as u64) as f64;
        let scaled = base * (px as f64 / nearest_px) * (fps as f64 / nearest_fps as f64);
        (scaled as u32).clamp(self.min_kbps, self.max_kbps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_is_monotonic() {
        for w in RESOLUTIONS.windows(2) {
            assert!(default_kbps(w[1], 30) > default_kbps(w[0], 30));
        }
        for f in FRAMERATES.windows(2) {
            assert!(default_kbps(1080, f[1]) > default_kbps(1080, f[0]));
        }
    }

    #[test]
    fn policy_clamps_and_overrides() {
        let mut p = StreamingPolicy {
            max_kbps: 8_000,
            ..Default::default()
        };
        assert_eq!(p.kbps_for(2160, 120), 8_000);
        p.overrides.insert("720p30".into(), 1_000);
        assert_eq!(p.kbps_for(720, 30), 1_000);
        assert_eq!(p.clamp_custom(12_345), 8_000);
        p.allow_custom_bitrate = false;
        assert_eq!(p.clamp_custom(100), 500);
    }

    #[test]
    fn hevc_only_for_4k_above_60() {
        let p = StreamingPolicy::default();
        assert_eq!(p.codec_for(2160, 120), Codec::H265);
        assert_eq!(p.codec_for(2160, 60), Codec::H264);
        assert_eq!(p.codec_for(1440, 120), Codec::H264);
        let off = StreamingPolicy {
            allow_h265: false,
            ..Default::default()
        };
        assert_eq!(off.codec_for(2160, 120), Codec::H264);
    }

    #[test]
    fn native_handles_ultrawide() {
        let p = StreamingPolicy {
            max_kbps: 50_000,
            ..Default::default()
        };
        let uw = p.native_kbps(3440, 1440, 60);
        let std = p.kbps_for(1440, 60);
        assert!(uw > std);
    }
}
