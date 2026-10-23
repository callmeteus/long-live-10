//! Agent surface shared by the command line and the Flutter desktop app.
//!
//! Diagnostics are read-only and never require administrator rights. The
//! Flutter app reaches this crate through `agent-ffi`, which serializes the
//! same types to UTF-8 JSON.

use serde::{Deserialize, Serialize};

/// Machine-readable diagnostics returned to the CLI, the UI, and the FFI layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostics {
    /// Version of the running agent build.
    pub agent_version: String,

    /// Operating system identifier reported by the Rust standard library.
    pub os: String,

    /// CPU architecture identifier reported by the Rust standard library.
    pub architecture: String,

    /// Windows version data, present only when running on Windows.
    pub windows: Option<WindowsVersion>,
}

/// Windows version data read from
/// `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowsVersion {
    /// Marketing product name, for example `Windows 10 Pro`.
    pub product_name: String,

    /// Feature update version, for example `22H2`.
    pub display_version: String,

    /// Build number, for example `19045`.
    pub build: String,

    /// Update Build Revision (UBR), for example `3803`.
    pub update_build_revision: Option<u32>,
}

impl Diagnostics {
    /// Serializes the diagnostics as compact JSON for the FFI boundary.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// Collects diagnostics without elevation and without modifying the system.
pub fn diagnostics() -> Diagnostics {
    Diagnostics {
        agent_version: env!("CARGO_PKG_VERSION").to_string(),
        os: std::env::consts::OS.to_string(),
        architecture: std::env::consts::ARCH.to_string(),
        windows: windows_version(),
    }
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
