//! Agent surface shared by the command line and the Flutter desktop app.
//!
//! Diagnostics are read-only and never require administrator rights. Planning,
//! apply, and revert are available through the `ll10-agent` command line. The
//! Flutter app reaches this crate through `agent-ffi`, which serializes the
//! same types to UTF-8 JSON.

pub mod cli;
pub mod executor;
pub mod request;

pub use ui_contract::{DiagnosticsPayload as Diagnostics, WindowsVersionPayload as WindowsVersion};

/// Collects diagnostics without elevation and without modifying the system.
pub fn diagnostics() -> Diagnostics {
    let windows = windows_version();
    let catalog_state = catalog_state(windows.as_ref());

    let mut info = Diagnostics::new(
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH,
        windows,
    );

    info.installed_patches = installed_patches();
    info.catalog_state = catalog_state;

    info
}

/// Reads the Windows version block from the registry.
#[cfg(windows)]
fn windows_version() -> Option<WindowsVersion> {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;

    let key = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion")
        .ok()?;

    let product_name = key.get_value("ProductName").ok()?;
    let display_version = key.get_value("DisplayVersion").ok()?;
    let build = key.get_value("CurrentBuildNumber").ok()?;
    let update_build_revision = key.get_value("UBR").ok();

    Some(WindowsVersion {
        product_name,
        display_version,
        build,
        update_build_revision,
    })
}

/// Non-Windows builds have no Windows version block.
#[cfg(not(windows))]
fn windows_version() -> Option<WindowsVersion> {
    None
}

/// Collects installed update identifiers without elevation.
#[cfg(windows)]
fn installed_patches() -> Vec<String> {
    use std::process::Command;

    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Get-HotFix | Select-Object -ExpandProperty HotFixID",
        ])
        .output();

    match output {
        Ok(result) if result.status.success() => String::from_utf8_lossy(&result.stdout)
            .lines()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}

/// Non-Windows hosts have no hotfix list.
#[cfg(not(windows))]
fn installed_patches() -> Vec<String> {
    Vec::new()
}

/// Describes catalog coverage for the detected Windows build.
fn catalog_state(windows: Option<&WindowsVersion>) -> String {
    match windows {
        Some(version) if version.build == "19045" => {
            "supported baseline: Windows 10 22H2 x64".to_string()
        }
        Some(_) => "build not in the supported catalog".to_string(),
        None => "not applicable".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_reports_build_identity() {
        let info = diagnostics();

        assert_eq!(info.agent_version, env!("CARGO_PKG_VERSION"));
        assert!(!info.os.is_empty());
        assert!(!info.architecture.is_empty());
    }

    #[test]
    fn diagnostics_round_trips_through_json() {
        let info = diagnostics();
        let json = info.to_json().expect("diagnostics serialize");
        let parsed: Diagnostics = serde_json::from_str(&json).expect("diagnostics parse");

        assert_eq!(parsed, info);
    }

    #[cfg(windows)]
    #[test]
    fn windows_version_is_available_on_windows() {
        let version = windows_version().expect("registry key is readable");
        let build: u32 = version.build.parse().expect("build number is numeric");

        assert!(build >= 10240, "unexpected build number {build}");
        assert!(!version.product_name.is_empty());
        assert!(!version.display_version.is_empty());
    }

    #[cfg(not(windows))]
    #[test]
    fn windows_version_is_absent_elsewhere() {
        assert_eq!(windows_version(), None);
    }
}
