mod host;
mod manifest;
mod plugin;

pub use host::*;
pub use manifest::*;
pub use plugin::*;

/// Plugin ABI generation. The loader refuses to load any plugin whose
/// `omniget_plugin_abi_version` export does not return exactly this value.
///
/// History:
/// - v3 (2026-07-09): invalidates every plugin built before the platform/model
///   types moved into `omniget-core` (PR #163) and before the toolchain
///   handshake existed. v2 plugins passed the handshake but crashed with
///   SIGSEGV at first call because Rust trait objects and std type layouts are
///   not ABI-stable across differing crate/compiler versions. v3 plugins must
///   also export `omniget_plugin_build_info` (added automatically by
///   `export_plugin!`), which the loader uses to reject plugins built by a
///   different rustc than the host.
/// - v2: pre-2026-07 interface.
pub const ABI_VERSION: u32 = 3;

/// NUL-terminated build fingerprint exported by `export_plugin!` through the
/// `omniget_plugin_build_info` C symbol and compared by the host at load time.
///
/// Format: `sdk=<omniget-plugin-sdk version>;rustc=<rustc --version output>`.
/// The rustc component (compiler version + commit hash) is what actually
/// gates loading: `ABI_VERSION` alone cannot detect two builds of the same
/// SDK made by different compilers, and Rust makes no ABI-stability promise
/// across compiler releases.
pub const BUILD_INFO: &str = concat!(
    "sdk=",
    env!("CARGO_PKG_VERSION"),
    ";rustc=",
    env!("OMNIGET_SDK_RUSTC_VERSION"),
    "\0"
);

/// [`BUILD_INFO`] without the trailing NUL, for logging and comparison.
pub fn build_info_str() -> &'static str {
    BUILD_INFO.trim_end_matches('\0')
}

/// Extracts the `rustc=` component from a build-info string.
pub fn rustc_component(info: &str) -> Option<&str> {
    info.split(';').find_map(|kv| kv.strip_prefix("rustc="))
}

/// Extracts the `sdk=` component from a build-info string.
pub fn sdk_component(info: &str) -> Option<&str> {
    info.split(';').find_map(|kv| kv.strip_prefix("sdk="))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_info_is_nul_terminated_c_string() {
        assert!(BUILD_INFO.ends_with('\0'));
        // exactly one NUL, so CStr::from_ptr on the host side sees the whole string
        assert_eq!(BUILD_INFO.matches('\0').count(), 1);
    }

    #[test]
    fn build_info_components_parse() {
        let info = build_info_str();
        assert_eq!(sdk_component(info), Some(env!("CARGO_PKG_VERSION")));
        let rustc = rustc_component(info).expect("rustc component present");
        // build.rs must have captured a real compiler version, e.g.
        // "rustc 1.96.0 (ac68faa20 2026-05-25)"
        assert!(
            rustc.starts_with("rustc "),
            "unexpected rustc fingerprint: {rustc}"
        );
    }

    #[test]
    fn mismatched_rustc_fingerprints_do_not_compare_equal() {
        let stale = "sdk=0.2.0;rustc=rustc 1.95.0 (59807616e 2026-04-10)";
        assert_ne!(rustc_component(stale), rustc_component(build_info_str()));
    }
}
