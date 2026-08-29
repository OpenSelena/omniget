//! The live e2e tests drive the real store, which resolves its directory from
//! `OMNIGET_OMNIDISC_SESSION_DIR` on every call. That is a process-global, so
//! two of these tests running on cargo's parallel threads clobber each other's
//! session directory and fail in ways that look like protocol bugs. They take
//! this lock instead of being marked `--test-threads=1`, which nothing enforces.

use std::sync::{Mutex, MutexGuard, OnceLock};

use super::store;

static LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub struct SessionDirGuard {
    _guard: MutexGuard<'static, ()>,
    previous: Option<String>,
}

impl SessionDirGuard {
    pub fn acquire() -> Self {
        let guard = LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        Self {
            _guard: guard,
            previous: std::env::var(store::SESSION_DIR_ENV).ok(),
        }
    }
}

impl Drop for SessionDirGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(dir) => std::env::set_var(store::SESSION_DIR_ENV, dir),
            None => std::env::remove_var(store::SESSION_DIR_ENV),
        }
    }
}
