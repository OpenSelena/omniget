//! Linux screen capture: xdg-desktop-portal ScreenCast → PipeWire.
//!
//! One path serves X11 and Wayland, because the portal is the only thing a
//! Wayland compositor will talk to and it works on X11 too. The consequence is
//! that the source picker is the desktop's own dialog: an unprivileged client
//! cannot enumerate windows on Wayland, so there is nothing to draw a grid of.
//! The share dialog says so instead of faking a list.

mod portal;
mod pw;

use super::{
    AudioSink, CaptureApi, CaptureGeometry, CaptureOptions, VideoSink, AUDIO_CHANNELS,
    AUDIO_SAMPLE_RATE,
};
use crate::stream::{AudioMode, SourceId, StreamError, StreamSource, StreamSources};

/// A frame as PipeWire handed it over: packed BGRx/BGRA rows, possibly padded.
/// `stride` is never assumed to be `width * 4` — mutter pads to 256 bytes.
pub struct CapturedFrame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub stride: usize,
    pub capture_us: i64,
}

impl CapturedFrame {
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn capture_us(&self) -> i64 {
        self.capture_us
    }
}

pub struct VideoCapture {
    inner: pw::StreamHandle,
}

impl VideoCapture {
    pub fn stop(self) {
        self.inner.stop();
    }
}

pub struct AudioCapture {
    inner: Option<pw::StreamHandle>,
}

impl AudioCapture {
    pub fn stop(self) {
        if let Some(inner) = self.inner {
            inner.stop();
        }
    }
}

pub struct Platform;

impl CaptureApi for Platform {
    fn list_sources(_thumbnails: bool) -> Result<StreamSources, StreamError> {
        // The portal's dialog is the picker. Answering with one placeholder is
        // honest; inventing a grid we cannot populate is not.
        portal::probe()?;
        Ok(StreamSources {
            displays: vec![StreamSource {
                id: SourceId::Display { id: 0 },
                title: "__omnidisc_portal_picker__".to_string(),
                app_name: None,
                width: 0,
                height: 0,
                thumbnail: None,
            }],
            windows: Vec::new(),
            apps: Vec::new(),
            app_audio_supported: false,
            system_audio_supported: true,
        })
    }

    fn thumbnail_for(_source: &SourceId) -> Option<String> {
        None
    }

    fn start_video(
        opts: &CaptureOptions,
        sink: VideoSink,
    ) -> Result<(VideoCapture, CaptureGeometry), StreamError> {
        if let SourceId::Synthetic { width, height } = opts.source {
            let (handle, geometry) = pw::start_synthetic(width, height, opts.fps, sink)?;
            return Ok((VideoCapture { inner: handle }, geometry));
        }
        let session = portal::open_screencast(opts.cursor)?;
        let (handle, geometry) = pw::start_video(session, opts, sink)?;
        Ok((VideoCapture { inner: handle }, geometry))
    }

    fn start_audio(
        mode: AudioMode,
        sink: AudioSink,
    ) -> Result<(AudioCapture, AudioMode), StreamError> {
        match mode {
            AudioMode::None => Ok((AudioCapture { inner: None }, AudioMode::None)),
            // Per-application capture needs a node registry and a picker of its
            // own; until that exists, asking for one app gets the honest answer
            // rather than silently recording everything.
            AudioMode::App { .. } | AudioMode::System => match pw::start_monitor_audio(sink) {
                Ok(handle) => Ok((
                    AudioCapture {
                        inner: Some(handle),
                    },
                    AudioMode::System,
                )),
                Err(e) => {
                    tracing::warn!("[omnidisc-media] linux screen audio unavailable: {e}");
                    Ok((AudioCapture { inner: None }, AudioMode::None))
                }
            },
        }
    }
}

/// Area-average downscale of a BGRx image. Only used when the share preset asks
/// for less than the screen's native height; capturing 4K and publishing 4K when
/// the user picked 1080p would spend the encoder budget on pixels nobody asked
/// for.
pub(crate) fn downscale_bgrx(
    src: &[u8],
    src_w: usize,
    src_h: usize,
    src_stride: usize,
    dst_w: usize,
    dst_h: usize,
) -> Vec<u8> {
    let mut out = vec![0u8; dst_w * dst_h * 4];
    if dst_w == 0 || dst_h == 0 {
        return out;
    }
    for y in 0..dst_h {
        let y0 = y * src_h / dst_h;
        let y1 = (((y + 1) * src_h).div_ceil(dst_h)).min(src_h).max(y0 + 1);
        for x in 0..dst_w {
            let x0 = x * src_w / dst_w;
            let x1 = (((x + 1) * src_w).div_ceil(dst_w)).min(src_w).max(x0 + 1);
            let (mut b, mut g, mut r, mut n) = (0u32, 0u32, 0u32, 0u32);
            for sy in y0..y1 {
                let row = sy * src_stride;
                for sx in x0..x1 {
                    let i = row + sx * 4;
                    if i + 2 >= src.len() {
                        continue;
                    }
                    b += src[i] as u32;
                    g += src[i + 1] as u32;
                    r += src[i + 2] as u32;
                    n += 1;
                }
            }
            let n = n.max(1);
            let o = (y * dst_w + x) * 4;
            out[o] = (b / n) as u8;
            out[o + 1] = (g / n) as u8;
            out[o + 2] = (r / n) as u8;
            out[o + 3] = 255;
        }
    }
    out
}

pub(crate) fn audio_format() -> (u32, u32) {
    (AUDIO_SAMPLE_RATE, AUDIO_CHANNELS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downscale_halves_and_averages() {
        // 2x2 image, one white pixel and three black, scaled to 1x1: the average
        // of the block, not a sample of one corner.
        let src = vec![
            255, 255, 255, 255, 0, 0, 0, 255, // row 0
            0, 0, 0, 255, 0, 0, 0, 255, // row 1
        ];
        let out = downscale_bgrx(&src, 2, 2, 8, 1, 1);
        assert_eq!(out.len(), 4);
        assert_eq!(out[0], 63);
        assert_eq!(out[3], 255);
    }

    #[test]
    fn downscale_respects_padded_stride() {
        // 1x2 visible pixels in rows padded to 12 bytes: the padding must not
        // bleed into the result.
        let mut src = vec![0u8; 24];
        src[0..4].copy_from_slice(&[10, 20, 30, 255]);
        src[12..16].copy_from_slice(&[10, 20, 30, 255]);
        let out = downscale_bgrx(&src, 1, 2, 12, 1, 1);
        assert_eq!(&out[0..3], &[10, 20, 30]);
    }
}
