//! Automatic yt-dlp install on first launch (or whenever it is missing).
//!
//! The app cannot download anything without yt-dlp, so instead of asking
//! the user to find an install button in Settings, the binary is fetched in
//! the background with retries. While offline it keeps waiting and installs
//! as soon as the network is back. The state is published to the UI so the
//! home banner can show progress and only reports a problem once every
//! attempt has failed.

use std::future::Future;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SetupState {
    /// Nothing started yet.
    Idle,
    /// Looking for an existing yt-dlp or starting the download.
    Installing { downloaded: u64, total: Option<u64> },
    /// An attempt failed; the next one starts in `retry_in_secs`.
    Waiting {
        attempt: u32,
        retry_in_secs: u64,
        offline: bool,
    },
    /// yt-dlp is ready to use.
    Ready,
    /// Every attempt failed (or the error cannot be fixed by retrying).
    Failed { offline: bool, error: String },
}

type Listener = Arc<dyn Fn(&SetupState) + Send + Sync>;

static STATE: RwLock<SetupState> = RwLock::new(SetupState::Idle);
static LISTENER: Mutex<Option<Listener>> = Mutex::new(None);
static RUNNING: AtomicBool = AtomicBool::new(false);
/// Wakes a waiting install early (the banner's "Retry now").
static WAKE: tokio::sync::Notify = tokio::sync::Notify::const_new();

/// Delays between attempts while the network seems to work.
const RETRY_DELAYS_SECS: &[u64] = &[3, 10, 30, 60];
/// While offline, try again this often...
const OFFLINE_POLL_SECS: u64 = 30;
/// ...for at most this many attempts (about two hours).
const OFFLINE_MAX_ATTEMPTS: u32 = 240;

pub fn status() -> SetupState {
    STATE.read().map(|s| s.clone()).unwrap_or(SetupState::Idle)
}

/// Called on every state change (the app forwards it to the window).
pub fn set_listener(listener: impl Fn(&SetupState) + Send + Sync + 'static) {
    if let Ok(mut l) = LISTENER.lock() {
        *l = Some(Arc::new(listener));
    }
}

fn publish(state: SetupState) {
    if let Ok(mut s) = STATE.write() {
        if *s == state {
            return;
        }
        *s = state.clone();
    }
    let listener = LISTENER.lock().ok().and_then(|l| l.clone());
    if let Some(listener) = listener {
        listener(&state);
    }
}

/// Download progress from the yt-dlp fetcher. Ignored unless an automatic
/// install is in progress, so a manual update in Settings does not change
/// the banner.
pub fn report_progress(downloaded: u64, total: Option<u64>) {
    let installing = matches!(status(), SetupState::Installing { .. });
    if installing && RUNNING.load(Ordering::SeqCst) {
        publish(SetupState::Installing { downloaded, total });
    }
}

/// True when the error comes from having no usable network (DNS, refused
/// or reset connection, timeout), as opposed to a bad file or a full disk.
pub fn is_offline_error(err: &anyhow::Error) -> bool {
    for cause in err.chain() {
        if let Some(e) = cause.downcast_ref::<reqwest::Error>() {
            if e.is_connect() || e.is_timeout() {
                return true;
            }
        }
        if let Some(e) = cause.downcast_ref::<std::io::Error>() {
            use std::io::ErrorKind::*;
            if matches!(
                e.kind(),
                ConnectionRefused | ConnectionReset | ConnectionAborted | TimedOut | NotConnected
            ) {
                return true;
            }
        }
    }
    let text = format!("{err:#}").to_ascii_lowercase();
    [
        "dns error",
        "failed to lookup address",
        "no such host",
        "network is unreachable",
        "error sending request",
        "connection refused",
        "timed out",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

/// Errors no retry can fix (sandbox without yt-dlp, no data directory).
fn is_permanent(err: &anyhow::Error) -> bool {
    let text = format!("{err:#}").to_ascii_lowercase();
    text.contains("flatpak sandbox") || text.contains("could not determine data directory")
}

/// Delay before attempt `attempt + 1`, or `None` to give up.
pub fn retry_delay(attempt: u32, offline: bool) -> Option<Duration> {
    if offline {
        return (attempt < OFFLINE_MAX_ATTEMPTS).then(|| Duration::from_secs(OFFLINE_POLL_SECS));
    }
    RETRY_DELAYS_SECS
        .get(attempt.saturating_sub(1) as usize)
        .map(|s| Duration::from_secs(*s))
}

/// Runs `install` until it succeeds or the retry policy gives up,
/// publishing each step. `sleep` is injected so tests do not wait.
pub async fn run_with_retry<I, IF, S, SF>(mut install: I, mut sleep: S) -> anyhow::Result<PathBuf>
where
    I: FnMut() -> IF,
    IF: Future<Output = anyhow::Result<PathBuf>>,
    S: FnMut(Duration) -> SF,
    SF: Future<Output = ()>,
{
    let mut attempt: u32 = 0;
    loop {
        attempt += 1;
        publish(SetupState::Installing {
            downloaded: 0,
            total: None,
        });
        let err = match install().await {
            Ok(path) => {
                publish(SetupState::Ready);
                return Ok(path);
            }
            Err(e) => e,
        };
        let offline = is_offline_error(&err);
        tracing::warn!("[ytdlp-setup] attempt {attempt} failed (offline: {offline}): {err:#}");
        let delay = if is_permanent(&err) {
            None
        } else {
            retry_delay(attempt, offline)
        };
        let Some(delay) = delay else {
            publish(SetupState::Failed {
                offline,
                error: format!("{err:#}"),
            });
            return Err(err);
        };
        publish(SetupState::Waiting {
            attempt,
            retry_in_secs: delay.as_secs(),
            offline,
        });
        sleep(delay).await;
    }
}

/// Makes sure yt-dlp is installed, in the background. Safe to call again
/// (the Retry button): a second call while one is running does nothing.
pub async fn ensure_in_background() {
    if RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }
    let _ = run_with_retry(
        || async {
            let path = crate::core::ytdlp::ensure_ytdlp().await?;
            crate::core::ytdlp::reset_ytdlp_cache();
            Ok(path)
        },
        |delay| async move {
            tokio::select! {
                _ = tokio::time::sleep(delay) => {}
                _ = WAKE.notified() => {}
            }
        },
    )
    .await;
    RUNNING.store(false, Ordering::SeqCst);
}

/// Skips the current wait of a running install. Returns false when no
/// install is running (the caller then starts one).
pub fn retry_now() -> bool {
    if RUNNING.load(Ordering::SeqCst) {
        WAKE.notify_one();
        true
    } else {
        false
    }
}

#[cfg(test)]
pub(crate) fn reset_for_tests() {
    if let Ok(mut s) = STATE.write() {
        *s = SetupState::Idle;
    }
    RUNNING.store(false, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU32;

    /// Tests share the global state, so they run one at a time.
    static SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    fn offline_error() -> anyhow::Error {
        anyhow::anyhow!("error sending request for url (https://github.com/): dns error")
    }

    #[tokio::test]
    async fn install_succeeds_after_temporary_failures() {
        let _g = SERIAL.lock().await;
        reset_for_tests();
        let seen: Arc<Mutex<Vec<SetupState>>> = Arc::default();
        let log = seen.clone();
        set_listener(move |s| log.lock().unwrap().push(s.clone()));
        let calls = AtomicU32::new(0);
        let slept: Mutex<Vec<u64>> = Mutex::default();
        let result = run_with_retry(
            || {
                let n = calls.fetch_add(1, Ordering::SeqCst);
                async move {
                    match n {
                        0 => Err(anyhow::anyhow!("yt-dlp: SHA-256 mismatch")),
                        1 => Err(offline_error()),
                        _ => Ok(PathBuf::from("/bin/yt-dlp")),
                    }
                }
            },
            |d| {
                slept.lock().unwrap().push(d.as_secs());
                async {}
            },
        )
        .await;
        assert_eq!(result.unwrap(), PathBuf::from("/bin/yt-dlp"));
        assert_eq!(calls.load(Ordering::SeqCst), 3);
        assert_eq!(*slept.lock().unwrap(), vec![3, OFFLINE_POLL_SECS]);
        assert_eq!(status(), SetupState::Ready);
        let states = seen.lock().unwrap().clone();
        assert!(states.contains(&SetupState::Waiting {
            attempt: 2,
            retry_in_secs: OFFLINE_POLL_SECS,
            offline: true
        }));
    }

    #[tokio::test]
    async fn gives_up_after_the_last_retry_and_reports_it() {
        let _g = SERIAL.lock().await;
        reset_for_tests();
        let calls = AtomicU32::new(0);
        let result = run_with_retry(
            || {
                calls.fetch_add(1, Ordering::SeqCst);
                async { Err::<PathBuf, _>(anyhow::anyhow!("HTTP 500")) }
            },
            |_| async {},
        )
        .await;
        assert!(result.is_err());
        assert_eq!(
            calls.load(Ordering::SeqCst),
            RETRY_DELAYS_SECS.len() as u32 + 1
        );
        assert!(matches!(
            status(),
            SetupState::Failed { offline: false, .. }
        ));
    }

    #[tokio::test]
    async fn a_sandbox_without_ytdlp_is_not_retried() {
        let _g = SERIAL.lock().await;
        reset_for_tests();
        let calls = AtomicU32::new(0);
        let _ = run_with_retry(
            || {
                calls.fetch_add(1, Ordering::SeqCst);
                async { Err::<PathBuf, _>(anyhow::anyhow!("yt-dlp not found in Flatpak sandbox")) }
            },
            |_| async {},
        )
        .await;
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn offline_keeps_waiting_much_longer() {
        assert_eq!(retry_delay(1, false), Some(Duration::from_secs(3)));
        assert_eq!(retry_delay(RETRY_DELAYS_SECS.len() as u32 + 1, false), None);
        assert!(retry_delay(100, true).is_some());
        assert_eq!(retry_delay(OFFLINE_MAX_ATTEMPTS, true), None);
    }

    #[test]
    fn network_errors_are_told_apart_from_bad_files() {
        assert!(is_offline_error(&offline_error()));
        let io = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused");
        assert!(is_offline_error(&anyhow::Error::new(io)));
        assert!(!is_offline_error(&anyhow::anyhow!(
            "SHA-256 mismatch for yt-dlp"
        )));
        assert!(!is_offline_error(&anyhow::anyhow!(
            "Access is denied. (os error 5)"
        )));
    }

    #[tokio::test]
    async fn progress_is_shown_only_during_the_automatic_install() {
        let _g = SERIAL.lock().await;
        reset_for_tests();
        report_progress(10, Some(100));
        assert_eq!(status(), SetupState::Idle);
        RUNNING.store(true, Ordering::SeqCst);
        publish(SetupState::Installing {
            downloaded: 0,
            total: None,
        });
        report_progress(10, Some(100));
        assert_eq!(
            status(),
            SetupState::Installing {
                downloaded: 10,
                total: Some(100)
            }
        );
        reset_for_tests();
    }
}
