//! Cookie Manager — multi-platform per-domain cookie storage.
//!
//! Lives on top of the legacy single-file `chrome-extension-cookies.txt` (see
//! `extension_storage`). The bridge endpoints (`/v1/enqueue`, `/v1/cookies`)
//! and the existing native message path continue to write the legacy file for
//! backwards compatibility with plugins compiled against SDK v2; this module
//! is invoked in parallel so plugins compiled against v3+ see the new layout.
//!
//! Module layout:
//! * `platform` — domain → `PlatformKind` mapping (drives UI logo + copy)
//! * `parsers`  — Netscape and JSON cookie import
//! * `storage`  — on-disk layout, `_meta.json` registry, transactional writes
//! * `commands` — Tauri commands consumed by Settings → Cookies UI

pub mod commands;
pub mod parsers;
pub mod platform;
pub mod storage;

pub use platform::{root_domain_of, PlatformKind};
pub use storage::{
    account_path_for_consumer, ingest_batch, load_registry, migrate_legacy_if_needed,
    AccountEntry, BucketEntry, CookieRegistry, IngestSource,
};
