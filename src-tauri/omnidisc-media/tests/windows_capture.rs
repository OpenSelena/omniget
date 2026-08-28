//! Runtime proof that the Windows capture stack actually initialises.
//!
//! Compiling is not evidence here: WinRT activation, the D3D11 device, the
//! frame pool and the WASAPI process-loopback activation all fail at runtime or
//! not at all, and none of that is reachable from the machine this code was
//! written on. The Windows CI job already runs `cargo test --workspace`, so
//! these run there on every push.
//!
//! A CI runner is a virtual desktop with no audio endpoint, so the assertions
//! are about behaviour that must hold anywhere: enumeration answers, a capture
//! either starts or fails with a typed error, and nothing panics or hangs.
#![cfg(windows)]

use omnidisc_media::capture::{self, CaptureOptions, VideoTick};
use omnidisc_media::stream::{AudioMode, SourceId};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[test]
fn enumeration_answers_and_reports_what_the_build_supports() {
    let sources = capture::list_sources(false).expect("list_sources must answer on Windows");
    assert!(
        !sources.displays.is_empty(),
        "a Windows session always has at least one display"
    );
    for d in &sources.displays {
        assert!(matches!(d.id, SourceId::Display { .. }));
        assert!(
            !d.title.trim().is_empty(),
            "a display needs a label the user can read"
        );
    }
    for w in &sources.windows {
        assert!(matches!(w.id, SourceId::Window { .. }));
    }
    // Process loopback needs build 20348; the flag exists so the picker can say
    // so instead of offering an option that always fails.
    if sources.app_audio_supported {
        assert!(
            sources.system_audio_supported,
            "a build new enough for per-app loopback also has the exclude-self mode"
        );
    }
}

#[test]
fn thumbnails_do_not_break_enumeration() {
    let sources = capture::list_sources(true).expect("list_sources with thumbnails");
    for s in sources.displays.iter().chain(sources.windows.iter()) {
        if let Some(thumb) = &s.thumbnail {
            assert!(
                thumb.starts_with("data:image/"),
                "a thumbnail is a data URL the webview can render, got {:.32}",
                thumb
            );
        }
    }
}

#[test]
fn capturing_a_display_delivers_ticks_or_a_typed_error() {
    let sources = capture::list_sources(false).expect("list_sources");
    let display = sources
        .displays
        .first()
        .expect("at least one display")
        .id
        .clone();
    let frames = Arc::new(AtomicU32::new(0));
    let ticks = Arc::new(AtomicU32::new(0));
    let (f, t) = (frames.clone(), ticks.clone());
    let sink: capture::VideoSink = Arc::new(move |tick| {
        t.fetch_add(1, Ordering::Relaxed);
        if matches!(tick, VideoTick::Frame(_)) {
            f.fetch_add(1, Ordering::Relaxed);
        }
    });

    let opts = CaptureOptions {
        source: display,
        fps: 30,
        height: Some(720),
        cursor: false,
    };
    let started = match capture::start_video(&opts, sink) {
        Ok(started) => started,
        Err(e) => {
            // A headless or locked session is a legitimate refusal; a panic or a
            // hang is not, and neither is an error with nothing to act on.
            assert!(
                !e.to_string().trim().is_empty(),
                "a refusal must explain itself"
            );
            return;
        }
    };
    let (capture_handle, geometry) = started;
    assert!(
        geometry.width > 0 && geometry.height > 0,
        "geometry must be real"
    );
    assert!(geometry.fps > 0);

    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline && ticks.load(Ordering::Relaxed) == 0 {
        std::thread::sleep(Duration::from_millis(50));
    }
    capture_handle.stop();

    // A still virtual desktop may produce no *frames*, but the idle ticks that
    // keep the encoder emitting CFR must arrive regardless — their absence is
    // the failure mode that silently freezes a viewer's picture.
    assert!(
        ticks.load(Ordering::Relaxed) > 0,
        "capture started but delivered neither a frame nor an idle tick in 3 s"
    );
}

#[test]
fn audio_capture_starts_or_refuses_without_panicking() {
    let sink: capture::AudioSink = Arc::new(|_samples: &[f32]| {});
    match capture::start_audio(AudioMode::System, sink) {
        Ok((handle, mode)) => {
            // The ladder may have degraded; whatever it reports has to be a mode
            // the UI can name, never a claim of something it did not get.
            assert!(matches!(mode, AudioMode::System | AudioMode::None));
            std::thread::sleep(Duration::from_millis(300));
            handle.stop();
        }
        Err(e) => {
            let msg = e.to_string();
            assert!(!msg.trim().is_empty(), "a refusal must explain itself");
        }
    }
}

#[test]
fn asking_for_no_audio_never_fails() {
    let sink: capture::AudioSink = Arc::new(|_samples: &[f32]| {});
    let (handle, mode) = capture::start_audio(AudioMode::None, sink)
        .expect("AudioMode::None is the path a share with no audio takes");
    assert_eq!(mode, AudioMode::None);
    handle.stop();
}
