//! Room key plumbing for voice E2EE (ADR 0014, spike S-05).
//!
//! The key itself is derived by the MLS layer — this module only carries it to
//! the SFU's frame cryptor and keeps the key-ring index in sync with the MLS
//! epoch. It is deliberately transport-free so the epoch → `set_shared_key`
//! step can be tested without a LiveKit room.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex as StdMutex};

/// LiveKit's default key ring holds 16 keys; the index travels in the SFrame
/// trailer, so a receiver that has not merged the newest MLS commit yet still
/// decodes the previous epoch's frames.
pub const KEY_RING_SIZE: u64 = 16;

/// The room key for one MLS epoch. `Debug` is hand-written so key material can
/// never reach a log line.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RoomKey {
    pub epoch: u64,
    pub key: [u8; 32],
}

impl RoomKey {
    pub fn new(epoch: u64, key: [u8; 32]) -> Self {
        Self { epoch, key }
    }

    pub fn ring_index(&self) -> i32 {
        (self.epoch % KEY_RING_SIZE) as i32
    }
}

impl std::fmt::Debug for RoomKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RoomKey").field("epoch", &self.epoch).finish_non_exhaustive()
    }
}

/// Whatever holds the shared key for the media path. LiveKit's `KeyProvider` is
/// the real one; tests use a recorder.
pub trait KeyRing: Send + Sync {
    fn set_shared_key(&self, key: &[u8], index: i32);
}

/// Applies MLS epochs to a key ring, exactly once per epoch.
#[derive(Default)]
pub struct KeyRotation {
    ring: StdMutex<Option<Arc<dyn KeyRing>>>,
    epoch: AtomicU64,
    armed: AtomicBool,
}

impl KeyRotation {
    pub fn new() -> Self {
        Self::default()
    }

    /// Bind a ring and push the first key. Called once per connection.
    pub fn arm(&self, ring: Arc<dyn KeyRing>, first: RoomKey) {
        if let Ok(mut slot) = self.ring.lock() {
            *slot = Some(ring.clone());
        }
        self.epoch.store(first.epoch, Ordering::Release);
        self.armed.store(true, Ordering::Release);
        ring.set_shared_key(&first.key, first.ring_index());
    }

    pub fn disarm(&self) {
        self.armed.store(false, Ordering::Release);
        if let Ok(mut slot) = self.ring.lock() {
            *slot = None;
        }
    }

    pub fn is_armed(&self) -> bool {
        self.armed.load(Ordering::Acquire)
    }

    pub fn epoch(&self) -> Option<u64> {
        self.is_armed().then(|| self.epoch.load(Ordering::Acquire))
    }

    /// Push a new epoch's key. Returns `true` when the ring was actually
    /// written — a repeated epoch is a no-op, and an unarmed rotation (no E2EE
    /// on this room) never writes.
    pub fn apply(&self, next: RoomKey) -> bool {
        if !self.is_armed() {
            return false;
        }
        if self.epoch.load(Ordering::Acquire) == next.epoch {
            return false;
        }
        let ring = match self.ring.lock() {
            Ok(slot) => slot.clone(),
            Err(_) => None,
        };
        let Some(ring) = ring else { return false };
        self.epoch.store(next.epoch, Ordering::Release);
        ring.set_shared_key(&next.key, next.ring_index());
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Recorder {
        calls: StdMutex<Vec<(i32, [u8; 32])>>,
    }

    impl Recorder {
        fn calls(&self) -> Vec<(i32, [u8; 32])> {
            self.calls.lock().map(|c| c.clone()).unwrap_or_default()
        }
    }

    impl KeyRing for Recorder {
        fn set_shared_key(&self, key: &[u8], index: i32) {
            let mut fixed = [0u8; 32];
            fixed[..key.len().min(32)].copy_from_slice(&key[..key.len().min(32)]);
            if let Ok(mut c) = self.calls.lock() {
                c.push((index, fixed));
            }
        }
    }

    fn key(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    #[test]
    fn arming_pushes_the_first_key_at_the_epochs_ring_index() {
        let recorder = Arc::new(Recorder::default());
        let rotation = KeyRotation::new();
        rotation.arm(recorder.clone(), RoomKey::new(3, key(1)));
        assert_eq!(recorder.calls(), vec![(3, key(1))]);
        assert_eq!(rotation.epoch(), Some(3));
    }

    #[test]
    fn every_epoch_change_writes_once_and_wraps_the_ring() {
        let recorder = Arc::new(Recorder::default());
        let rotation = KeyRotation::new();
        rotation.arm(recorder.clone(), RoomKey::new(15, key(1)));
        assert!(rotation.apply(RoomKey::new(16, key(2))));
        assert!(!rotation.apply(RoomKey::new(16, key(2))), "the same epoch must not rewrite the ring");
        assert!(rotation.apply(RoomKey::new(17, key(3))));
        assert_eq!(recorder.calls(), vec![(15, key(1)), (0, key(2)), (1, key(3))]);
    }

    #[test]
    fn an_unarmed_rotation_never_touches_the_ring() {
        let rotation = KeyRotation::new();
        assert!(!rotation.is_armed());
        assert!(!rotation.apply(RoomKey::new(1, key(9))));
        assert_eq!(rotation.epoch(), None);
    }

    #[test]
    fn disarming_stops_further_writes() {
        let recorder = Arc::new(Recorder::default());
        let rotation = KeyRotation::new();
        rotation.arm(recorder.clone(), RoomKey::new(0, key(1)));
        rotation.disarm();
        assert!(!rotation.apply(RoomKey::new(1, key(2))));
        assert_eq!(recorder.calls().len(), 1);
    }

    #[test]
    fn the_debug_output_never_carries_key_material() {
        let printed = format!("{:?}", RoomKey::new(7, key(0xAB)));
        assert!(printed.contains('7'));
        assert!(!printed.contains("171") && !printed.to_lowercase().contains("ab"), "{printed}");
    }
}
