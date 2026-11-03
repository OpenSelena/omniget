//! Turns a multi-image post with a soundtrack (TikTok photo mode and the
//! like) into one vertical video: a slow Ken Burns zoom on each image, short
//! crossfades, and a total length that matches the audio. The original
//! images stay on disk next to the video.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

static ENABLED: AtomicBool = AtomicBool::new(true);

/// Set from the "Make slideshow videos" setting at startup and on every save.
pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

pub const WIDTH: u32 = 1080;
pub const HEIGHT: u32 = 1920;
pub const FPS: u32 = 30;
/// Crossfade length between two images, in seconds.
pub const FADE_SECS: f64 = 0.5;
/// Shortest time an image stays on screen, in seconds.
pub const MIN_SLIDE_SECS: f64 = 1.5;
/// Longest video made when the audio length is unknown or very long.
pub const MAX_TOTAL_SECS: f64 = 600.0;

/// Seconds each image is shown so that the video, crossfades included,
/// lasts as long as the audio.
pub fn slide_secs(images: usize, audio_secs: f64) -> f64 {
    let n = images.max(1) as f64;
    let total = if audio_secs.is_finite() && audio_secs > 0.0 {
        audio_secs.min(MAX_TOTAL_SECS)
    } else {
        n * 3.0
    };
    ((total + (n - 1.0) * FADE_SECS) / n).max(MIN_SLIDE_SECS)
}

/// The ffmpeg filtergraph for `images` single-frame inputs (0..n) of
/// `slide` seconds each. Audio is input `images`.
pub fn filter_graph(images: usize, slide: f64) -> String {
    let frames = (slide * FPS as f64).round().max(1.0) as u64;
    let mut parts = Vec::with_capacity(images * 2);
    for i in 0..images {
        // Even zoom-in or zoom-out per slide, centred, 2x supersampled so the
        // zoom does not jitter.
        let zoom = if i % 2 == 0 {
            format!("min(1+0.10*on/{frames},1.10)")
        } else {
            format!("max(1.10-0.10*on/{frames},1.0)")
        };
        parts.push(format!(
            "[{i}:v]scale={w2}:{h2}:force_original_aspect_ratio=decrease,\
             pad={w2}:{h2}:(ow-iw)/2:(oh-ih)/2:color=black,setsar=1,\
             zoompan=z='{zoom}':d={frames}:x='iw/2-(iw/zoom/2)':y='ih/2-(ih/zoom/2)':\
             s={w}x{h}:fps={fps},format=yuv420p[v{i}]",
            w2 = WIDTH * 2,
            h2 = HEIGHT * 2,
            w = WIDTH,
            h = HEIGHT,
            fps = FPS,
        ));
    }
    if images == 1 {
        parts.push("[v0]null[vout]".to_string());
    } else {
        let mut prev = "v0".to_string();
        for i in 1..images {
            let offset = i as f64 * (slide - FADE_SECS);
            let out = if i == images - 1 {
                "vout".to_string()
            } else {
                format!("x{i}")
            };
            parts.push(format!(
                "[{prev}][v{i}]xfade=transition=fade:duration={FADE_SECS}:offset={offset:.3}[{out}]"
            ));
            prev = out;
        }
    }
    parts.join(";")
}

/// Full ffmpeg argument list (without the binary).
pub fn ffmpeg_args(images: &[PathBuf], audio: &Path, audio_secs: f64, out: &Path) -> Vec<String> {
    let slide = slide_secs(images.len(), audio_secs);
    let total = images.len() as f64 * slide - (images.len().saturating_sub(1)) as f64 * FADE_SECS;
    let mut args: Vec<String> = vec![
        "-y".into(),
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
    ];
    for img in images {
        args.push("-i".into());
        args.push(img.to_string_lossy().to_string());
    }
    args.push("-i".into());
    args.push(audio.to_string_lossy().to_string());
    args.extend([
        "-filter_complex".to_string(),
        filter_graph(images.len(), slide),
        "-map".into(),
        "[vout]".into(),
        "-map".into(),
        format!("{}:a:0", images.len()),
        "-t".into(),
        format!("{total:.3}"),
        "-c:v".into(),
        "libx264".into(),
        "-preset".into(),
        "veryfast".into(),
        "-crf".into(),
        "20".into(),
        "-pix_fmt".into(),
        "yuv420p".into(),
        "-r".into(),
        FPS.to_string(),
        "-c:a".into(),
        "aac".into(),
        "-b:a".into(),
        "192k".into(),
        "-movflags".into(),
        "+faststart".into(),
    ]);
    args.push(out.to_string_lossy().to_string());
    args
}

/// Builds the slideshow video. Returns its path.
pub async fn build(images: &[PathBuf], audio: &Path, out: &Path) -> anyhow::Result<PathBuf> {
    if images.is_empty() {
        anyhow::bail!("no images for the slideshow");
    }
    let ffmpeg = crate::core::dependencies::ensure_ffmpeg().await?;
    let audio_secs = crate::core::ffmpeg::get_duration_us(audio)
        .await
        .map(|us| us as f64 / 1_000_000.0)
        .unwrap_or(0.0);
    let output = crate::core::process::command(&ffmpeg)
        .args(ffmpeg_args(images, audio, audio_secs, out))
        .output()
        .await?;
    if !output.status.success() {
        let _ = tokio::fs::remove_file(out).await;
        anyhow::bail!(
            "ffmpeg could not build the slideshow: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(out.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slides_fill_the_audio_with_crossfades() {
        let d = slide_secs(4, 12.0);
        let total = 4.0 * d - 3.0 * FADE_SECS;
        assert!((total - 12.0).abs() < 1e-9, "{total}");
    }

    #[test]
    fn slides_have_a_minimum_and_a_default() {
        assert_eq!(slide_secs(30, 5.0), MIN_SLIDE_SECS);
        assert!((slide_secs(2, 0.0) - (6.0 + FADE_SECS) / 2.0).abs() < 1e-9);
        assert!(slide_secs(1, f64::NAN) >= MIN_SLIDE_SECS);
    }

    #[test]
    fn graph_chains_xfades_into_vout() {
        let g = filter_graph(3, 4.0);
        assert!(g.contains("[0:v]") && g.contains("[2:v]"));
        assert!(g.contains("[v0][v1]xfade=transition=fade:duration=0.5:offset=3.500[x1]"));
        assert!(g.contains("[x1][v2]xfade=transition=fade:duration=0.5:offset=7.000[vout]"));
        assert!(g.contains("s=1080x1920"));
        assert!(filter_graph(1, 3.0).ends_with("[v0]null[vout]"));
    }

    #[test]
    fn args_map_audio_and_limit_length() {
        let imgs = vec![PathBuf::from("a.jpg"), PathBuf::from("b.jpg")];
        let a = ffmpeg_args(&imgs, Path::new("m.mp3"), 10.0, Path::new("o.mp4"));
        let j = a.join(" ");
        assert!(j.contains("-map [vout] -map 2:a:0 -t 10.000"), "{j}");
        assert!(j.contains("-pix_fmt yuv420p") && j.contains("-movflags +faststart"));
        assert_eq!(a.last().unwrap(), "o.mp4");
    }
}

#[cfg(test)]
mod ffmpeg_tests {
    use super::*;

    /// Runs the real graph when ffmpeg is on PATH (skipped otherwise).
    #[test]
    fn real_ffmpeg_builds_a_matching_video() {
        let Ok(ff) = which_ffmpeg() else { return };
        let dir = std::env::temp_dir().join(format!("slideshow-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut imgs = Vec::new();
        for (i, size) in ["800x600", "600x900", "1080x1920"].iter().enumerate() {
            let p = dir.join(format!("{i}.jpg"));
            let ok = std::process::Command::new(&ff)
                .args(["-y", "-loglevel", "error", "-f", "lavfi", "-i"])
                .arg(format!("testsrc=size={size}:duration=1"))
                .args(["-frames:v", "1"])
                .arg(&p)
                .status()
                .unwrap()
                .success();
            assert!(ok);
            imgs.push(p);
        }
        let audio = dir.join("a.m4a");
        assert!(std::process::Command::new(&ff)
            .args([
                "-y",
                "-loglevel",
                "error",
                "-f",
                "lavfi",
                "-i",
                "sine=d=5",
                "-c:a",
                "aac"
            ])
            .arg(&audio)
            .status()
            .unwrap()
            .success());
        let out = dir.join("out.mp4");
        let status = std::process::Command::new(&ff)
            .args(ffmpeg_args(&imgs, &audio, 5.0, &out))
            .status()
            .unwrap();
        assert!(status.success());
        let probe = std::process::Command::new(ff.with_file_name("ffprobe"))
            .args(["-v", "error", "-select_streams", "v:0", "-show_entries"])
            .args(["stream=width,height:format=duration", "-of", "csv=p=0"])
            .arg(&out)
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&probe.stdout);
        assert!(text.contains("1080,1920"), "{text}");
        let dur: f64 = text.lines().last().unwrap().trim().parse().unwrap();
        assert!((dur - 5.0).abs() < 0.3, "{dur}");
        let _ = std::fs::remove_dir_all(dir);
    }

    fn which_ffmpeg() -> Result<PathBuf, ()> {
        let ok = std::process::Command::new("ffmpeg")
            .arg("-version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !ok {
            return Err(());
        }
        for dir in std::env::var("PATH").unwrap_or_default().split(':') {
            let p = Path::new(dir).join("ffmpeg");
            if p.is_file() {
                return Ok(p);
            }
        }
        Err(())
    }
}
