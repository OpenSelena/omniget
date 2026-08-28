fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-link-arg=-ObjC");
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
        if let Ok(out) = std::process::Command::new("xcode-select").arg("-p").output() {
            let xcode = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !xcode.is_empty() {
                println!("cargo:rustc-link-arg=-Wl,-rpath,{xcode}/Toolchains/XcodeDefault.xctoolchain/usr/lib/swift-5.5/macosx");
            }
        }
    }
}
