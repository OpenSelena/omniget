use std::process::Command;

// Capture the exact rustc that compiles this crate. Rust has no stable ABI:
// trait-object vtables, fat-pointer calling conventions and std type layouts
// may all change between compiler releases, so a plugin dylib is only safe to
// load into a host built by the *same* rustc. The version string (including
// the commit hash) is embedded in BUILD_INFO and exchanged through the
// `omniget_plugin_build_info` handshake symbol at load time.
fn main() {
    // Cargo sets RUSTC to the compiler used for the build.
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let version = Command::new(&rustc)
        .arg("--version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=OMNIGET_SDK_RUSTC_VERSION={version}");
    println!("cargo:rerun-if-env-changed=RUSTC");
    println!("cargo:rerun-if-env-changed=RUSTUP_TOOLCHAIN");
}
