use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

/// Desktop launches can select Intel Homebrew's old CLI, which cannot read
/// the native CLI's MSAL cache. Configure this before any threads start.
pub fn configure_azure_cli() {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    if let Some(path) = prefer_native_cli(
        &std::env::var_os("PATH").unwrap_or_default(),
        Path::new("/opt/homebrew/bin"),
        Path::new("/usr/local/bin"),
    ) {
        std::env::set_var("PATH", path);
    }
}

fn prefer_native_cli(path: &OsStr, native: &Path, legacy: &Path) -> Option<OsString> {
    if !native.join("az").is_file() {
        return None;
    }
    let paths: Vec<PathBuf> = std::env::split_paths(path).collect();
    // Preserve deliberately selected custom installations.
    if paths.iter().find(|dir| dir.join("az").is_file()).is_some_and(|dir| dir != legacy) {
        return None;
    }
    std::env::join_paths(std::iter::once(native.to_path_buf()).chain(
        paths.into_iter().filter(|dir| dir != native),
    )).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_cli_replaces_legacy_but_respects_custom_installs() {
        let root = std::env::temp_dir().join(format!("cdw-cli-path-{}", std::process::id()));
        let native = root.join("native");
        let legacy = root.join("legacy");
        let custom = root.join("custom");
        for dir in [&native, &legacy, &custom] {
            std::fs::create_dir_all(dir).unwrap();
            std::fs::write(dir.join("az"), "fixture").unwrap();
        }
        let old = std::env::join_paths([&legacy, &native]).unwrap();
        let fixed = prefer_native_cli(&old, &native, &legacy).unwrap();
        assert_eq!(std::env::split_paths(&fixed).collect::<Vec<_>>(), vec![native.clone(), legacy.clone()]);
        let selected = std::env::join_paths([&custom, &legacy]).unwrap();
        assert!(prefer_native_cli(&selected, &native, &legacy).is_none());
        assert!(prefer_native_cli(OsStr::new(""), &native, &legacy).is_some());
        assert!(prefer_native_cli(&old, &root.join("absent"), &legacy).is_none());
        std::fs::remove_dir_all(root).unwrap();
    }
}
