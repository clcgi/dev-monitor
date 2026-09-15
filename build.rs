//! Embeds the SBM logo only when `assets/sbm-logo.svg` is present, so a checkout without it still builds.
fn main() {
    println!("cargo::rustc-check-cfg=cfg(has_logo)");
    println!("cargo::rerun-if-changed=assets/sbm-logo.svg");
    println!("cargo::rerun-if-changed=assets");
    if std::path::Path::new("assets/sbm-logo.svg").is_file() {
        println!("cargo::rustc-cfg=has_logo");
    }
}
