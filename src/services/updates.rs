//! Checking for, downloading and handing off a newer release.

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Where releases are published.
const RELEASES_API: &str = "https://api.github.com/repos/clcgi/dev-monitor/releases/latest";

/// GitHub rejects a request with no User-Agent, with an unhelpful 403.
const USER_AGENT: &str = concat!("dev-monitor/", env!("CARGO_PKG_VERSION"));

/// Bounded: nothing waits on this check.
const CHECK_TIMEOUT_S: u64 = 8;

/// The installer is ~25 MB, so this is generous rather than tight.
const DOWNLOAD_TIMEOUT_S: u64 = 300;

/// The version this binary was built as.
pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// The installer extension this platform can actually run.
///
/// `None` elsewhere: the release publishes one `.dmg` and one `.exe`, so a
/// Linux build has nothing to offer and must not download a file it cannot use.
pub fn installer_ext() -> Option<&'static str> {
    match std::env::consts::OS {
        "macos" => Some("dmg"),
        "windows" => Some("exe"),
        _ => None,
    }
}

#[derive(Deserialize)]
struct ApiAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

#[derive(Deserialize)]
struct ApiRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    assets: Vec<ApiAsset>,
}

/// The installer for this platform, when the release carries one.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Installer {
    pub name: String,
    pub url: String,
    /// Declared by the API, and compared against the bytes received.
    pub size: u64,
}

/// A release newer than the running binary.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Update {
    /// Without the leading `v`, so it reads next to the current version.
    pub version: String,
    /// The release page. The fallback when there is no installer to fetch.
    pub url: String,
    pub installer: Option<Installer>,
}

/// Compare two dotted versions numerically.
fn is_newer(candidate: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u64> {
        v.trim_start_matches('v')
            .split(['.', '-', '+'])
            .map(|part| part.parse::<u64>().unwrap_or(0))
            .collect()
    };
    let (a, b) = (parse(candidate), parse(current));
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (a.get(i).copied().unwrap_or(0), b.get(i).copied().unwrap_or(0));
        if x != y {
            return x > y;
        }
    }
    false
}

/// Pick the asset this platform can run, by extension.
fn pick_installer(assets: &[ApiAsset], ext: Option<&str>) -> Option<Installer> {
    let ext = ext?;
    let suffix = format!(".{ext}");
    assets
        .iter()
        .find(|a| a.name.to_ascii_lowercase().ends_with(&suffix))
        .map(|a| Installer {
            name: a.name.clone(),
            url: a.browser_download_url.clone(),
            size: a.size,
        })
}

/// Ask GitHub for the latest release.
pub async fn check() -> Option<Update> {
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(CHECK_TIMEOUT_S))
        .build()
        .ok()?;

    let release: ApiRelease = client.get(RELEASES_API).send().await.ok()?.json().await.ok()?;

    is_newer(&release.tag_name, current_version()).then(|| Update {
        version: release.tag_name.trim_start_matches('v').to_string(),
        installer: pick_installer(&release.assets, installer_ext()),
        url: release.html_url,
    })
}

/// Where a downloaded installer is written.
///
/// The system temp directory, under our own subdirectory so a stale download is
/// identifiable and the OS reclaims it. Deliberately NOT next to the running
/// binary: that is inside the app bundle on macOS and under Program Files on
/// Windows, neither of which is writable by an ordinary user.
fn download_dir() -> PathBuf {
    std::env::temp_dir().join("dev-monitor-updates")
}

/// Download the installer and return where it landed.
///
/// The size is checked against what the API declared. That catches the
/// realistic failure -- a truncated or interrupted download -- and it is worth
/// being clear about what it does NOT do: it is not an authenticity check.
/// Authenticity here rests on TLS and on GitHub serving the release; the
/// installers are unsigned, so the OS will say so when they run.
pub async fn download(installer: &Installer) -> Result<PathBuf, String> {
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(DOWNLOAD_TIMEOUT_S))
        .build()
        .map_err(|e| e.to_string())?;

    let bytes = client
        .get(&installer.url)
        .send()
        .await
        .map_err(|e| format!("download failed: {e}"))?
        .error_for_status()
        .map_err(|e| format!("download refused: {e}"))?
        .bytes()
        .await
        .map_err(|e| format!("download interrupted: {e}"))?;

    if installer.size != 0 && bytes.len() as u64 != installer.size {
        return Err(format!(
            "expected {} bytes, received {} -- the download was truncated",
            installer.size,
            bytes.len()
        ));
    }

    let dir = download_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    let path = dir.join(&installer.name);
    std::fs::write(&path, &bytes).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    Ok(path)
}

/// Hand the downloaded installer to the operating system.
///
/// NOT AN IN-PLACE REPLACEMENT, deliberately. On Windows the `.exe` is an NSIS
/// installer and running it is the whole job. On macOS a `.dmg` is a disk image:
/// `open` mounts it and shows the volume, and the user drags the app across.
/// Copying over the running `/Applications` bundle ourselves would mean deleting
/// the quarantine flag on an unsigned download, which is the one check standing
/// between the user and an app nobody verified.
pub fn launch(path: &Path) -> Result<(), String> {
    let result = match std::env::consts::OS {
        "windows" => std::process::Command::new(path).spawn().map(|_| ()),
        // `open` mounts the image and reveals it; it does not block.
        "macos" => std::process::Command::new("open").arg(path).spawn().map(|_| ()),
        _ => return Err("no installer for this platform".to_string()),
    };
    result.map_err(|e| format!("could not start {}: {e}", path.display()))
}

/// True when running the installer should close this app first.
///
/// Windows only: the installer overwrites files this process holds open, and a
/// locked file fails late with a message about permissions. A mounted `.dmg`
/// touches nothing, so macOS keeps running.
pub fn must_quit_to_install() -> bool {
    std::env::consts::OS == "windows"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(name: &str, size: u64) -> ApiAsset {
        ApiAsset {
            name: name.to_string(),
            browser_download_url: format!("https://example.invalid/{name}"),
            size,
        }
    }

    #[test]
    fn a_higher_version_is_newer() {
        assert!(is_newer("v0.3.0", "0.2.0"));
        assert!(is_newer("0.2.1", "0.2.0"));
        assert!(is_newer("v1.0.0", "0.9.9"));
    }

    #[test]
    fn the_same_version_is_not_newer() {
        // The common case by far -- everyone running the current release must see.
        assert!(!is_newer("v0.2.0", "0.2.0"));
        assert!(!is_newer("0.2.0", "0.2.0"));
    }

    #[test]
    fn an_older_version_is_not_newer() {
        assert!(!is_newer("v0.1.0", "0.2.0"));
        assert!(!is_newer("v0.2.0", "0.10.0"));
    }

    #[test]
    fn double_digit_segments_compare_numerically() {
        // A string compare puts "0.10.0" before "0.9.0" and would skip the release.
        assert!(is_newer("v0.10.0", "0.9.0"));
        assert!(!is_newer("v0.9.0", "0.10.0"));
        assert!(is_newer("v1.11.0", "1.9.3"));
    }

    #[test]
    fn the_v_prefix_is_optional_on_either_side() {
        assert!(is_newer("v0.3.0", "0.2.0"));
        assert!(is_newer("0.3.0", "v0.2.0"));
    }

    #[test]
    fn a_shorter_version_is_padded_rather_than_treated_as_greater() {
        assert!(!is_newer("v0.2", "0.2.0"));
        assert!(is_newer("v0.3", "0.2.9"));
    }

    #[test]
    fn a_non_numeric_segment_does_not_panic() {
        // A background check must never take the app down over a hand-typed tag.
        assert!(!is_newer("v0.2.0-rc1", "0.2.0"));
        assert!(is_newer("v0.3.0-rc1", "0.2.0"));
        assert!(!is_newer("nonsense", "0.2.0"));
    }

    #[test]
    fn each_platform_is_offered_only_the_installer_it_can_run() {
        // Handing a .dmg to Windows is not a smaller mistake than handing it
        // nothing: it downloads 25 MB the machine cannot open.
        let assets = [asset("dev-monitor.dmg", 100), asset("dev-monitor.exe", 200)];
        assert_eq!(pick_installer(&assets, Some("dmg")).unwrap().name, "dev-monitor.dmg");
        assert_eq!(pick_installer(&assets, Some("exe")).unwrap().name, "dev-monitor.exe");
    }

    #[test]
    fn a_platform_with_no_installer_gets_none() {
        let assets = [asset("dev-monitor.dmg", 100)];
        assert!(pick_installer(&assets, None).is_none());
    }

    #[test]
    fn a_missing_asset_is_none_rather_than_the_wrong_one() {
        // The release publishes one .dmg and one .exe. If a job failed and only
        // one was attached, the other platform must fall back to the release
        // page -- never to whatever asset happens to be first.
        let assets = [asset("dev-monitor.dmg", 100)];
        assert!(pick_installer(&assets, Some("exe")).is_none());
    }

    #[test]
    fn the_extension_match_is_case_insensitive() {
        let assets = [asset("DevMonitor-Setup.EXE", 200)];
        assert_eq!(pick_installer(&assets, Some("exe")).unwrap().name, "DevMonitor-Setup.EXE");
    }

    #[test]
    fn an_asset_is_matched_on_its_extension_not_on_the_name_containing_it() {
        // `dev-monitor.exe.sha256` ends with .sha256, and a `contains` test
        // would match it for a Windows user and download a checksum file.
        let assets = [asset("dev-monitor.exe.sha256", 64), asset("dev-monitor.exe", 200)];
        assert_eq!(pick_installer(&assets, Some("exe")).unwrap().name, "dev-monitor.exe");
    }

    #[test]
    fn downloads_land_outside_the_application_directory() {
        // The running binary lives inside the .app bundle on macOS and under
        // Program Files on Windows; neither is writable by an ordinary user.
        let dir = download_dir();
        assert!(dir.starts_with(std::env::temp_dir()));
        assert!(dir.ends_with("dev-monitor-updates"));
    }
}
