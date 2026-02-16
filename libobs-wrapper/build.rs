fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "windows" && target_os != "macos" {
        println!("cargo:rustc-link-lib=X11");
        println!("cargo:rustc-link-lib=wayland-client");
    }
}
