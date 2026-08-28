use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub const OMNIDISC_EPOCH_MS: u64 = 1_785_542_400_000;
pub const BUCKET_SPAN_MS: u64 = 10 * 24 * 60 * 60 * 1000;

const WORKER_BITS: u64 = 10;
const SEQUENCE_BITS: u64 = 12;
const WORKER_SHIFT: u64 = SEQUENCE_BITS;
const TIMESTAMP_SHIFT: u64 = WORKER_BITS + SEQUENCE_BITS;
const SEQUENCE_MASK: u64 = (1 << SEQUENCE_BITS) - 1;
const WORKER_MASK: u64 = (1 << WORKER_BITS) - 1;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Snowflake(pub u64);

impl Snowflake {
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    pub fn timestamp_ms(self) -> u64 {
        (self.0 >> TIMESTAMP_SHIFT) + OMNIDISC_EPOCH_MS
    }

    pub fn worker_id(self) -> u16 {
        ((self.0 >> WORKER_SHIFT) & WORKER_MASK) as u16
    }

    pub fn sequence(self) -> u16 {
        (self.0 & SEQUENCE_MASK) as u16
    }

    pub fn bucket(self) -> u32 {
        ((self.timestamp_ms() - OMNIDISC_EPOCH_MS) / BUCKET_SPAN_MS) as u32
    }

    pub fn bucket_for_timestamp_ms(ts: u64) -> u32 {
        (ts.saturating_sub(OMNIDISC_EPOCH_MS) / BUCKET_SPAN_MS) as u32
    }

    pub fn lower_bound_for_bucket(bucket: u32) -> Self {
        Self((bucket as u64 * BUCKET_SPAN_MS) << TIMESTAMP_SHIFT)
    }

    pub fn is_nil(self) -> bool {
        self.0 == 0
    }
}

impl fmt::Debug for Snowflake {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Snowflake({})", self.0)
    }
}

impl fmt::Display for Snowflake {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Snowflake {
    type Err = std::num::ParseIntError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<u64>().map(Snowflake)
    }
}

impl Serialize for Snowflake {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Snowflake {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl From<u64> for Snowflake {
    fn from(v: u64) -> Self {
        Self(v)
    }
}

pub struct SnowflakeGenerator {
    worker_id: u64,
    state: AtomicU64,
}

impl SnowflakeGenerator {
    pub fn new(worker_id: u16) -> Self {
        Self {
            worker_id: (worker_id as u64) & WORKER_MASK,
            state: AtomicU64::new(0),
        }
    }

    pub fn next(&self) -> Snowflake {
        loop {
            let now = Self::now_ms();
            let prev = self.state.load(Ordering::Acquire);
            let prev_ts = prev >> SEQUENCE_BITS;
            let prev_seq = prev & SEQUENCE_MASK;
            let (ts, seq) = if now > prev_ts {
                (now, 0)
            } else if prev_seq < SEQUENCE_MASK {
                (prev_ts, prev_seq + 1)
            } else {
                std::hint::spin_loop();
                continue;
            };
            let next = (ts << SEQUENCE_BITS) | seq;
            if self
                .state
                .compare_exchange(prev, next, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                return Snowflake((ts << TIMESTAMP_SHIFT) | (self.worker_id << WORKER_SHIFT) | seq);
            }
        }
    }

    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
            .saturating_sub(OMNIDISC_EPOCH_MS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_monotonic_and_unique() {
        let g = SnowflakeGenerator::new(3);
        let mut prev = g.next();
        for _ in 0..10_000 {
            let n = g.next();
            assert!(n > prev);
            assert_eq!(n.worker_id(), 3);
            prev = n;
        }
    }

    #[test]
    fn bucket_matches_timestamp_division() {
        let g = SnowflakeGenerator::new(0);
        let id = g.next();
        assert_eq!(id.bucket(), Snowflake::bucket_for_timestamp_ms(id.timestamp_ms()));
        assert!(Snowflake::lower_bound_for_bucket(id.bucket()) <= id);
        assert!(Snowflake::lower_bound_for_bucket(id.bucket() + 1) > id);
    }

    #[test]
    fn serializes_as_string() {
        let id = Snowflake(1234567890123);
        assert_eq!(serde_json::to_string(&id).unwrap(), "\"1234567890123\"");
        let back: Snowflake = serde_json::from_str("\"1234567890123\"").unwrap();
        assert_eq!(back, id);
    }
}
