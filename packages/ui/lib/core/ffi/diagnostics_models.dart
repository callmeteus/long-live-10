import "contract_version.dart";

/// Typed view of the JSON payload returned by `ll10_diagnostics_json`.
class Diagnostics {
    const Diagnostics({
        this.contractVersion = const ContractVersion.current(),
        required this.agentVersion,
        required this.os,
        required this.architecture,
        this.windows,
        this.installedPatches = const <String>[],
        this.catalogState = "unknown",
    });

    /// Parses the payload produced by the Rust agent.
    factory Diagnostics.fromJson(Map<String, dynamic> json) {
        final contractVersion = ContractVersion.fromJson(
            json["contract_version"],
        );
        contractVersion.ensureSupported();

        final windows = json["windows"];

        if (windows != null && windows is! Map<String, dynamic>) {
            throw const FormatException("\"windows\" must be an object or null.");
        }

        final installedPatches = <String>[];
        final rawPatches = json["installed_patches"];
        if (rawPatches != null) {
            if (rawPatches is! List) {
                throw const FormatException("\"installed_patches\" must be a list.");
            }
            for (final value in rawPatches) {
                if (value is! String || value.isEmpty) {
                    throw const FormatException(
                        "\"installed_patches\" must contain non-empty strings.",
                    );
                }
                installedPatches.add(value);
            }
        }

        final catalogState = json["catalog_state"];
        if (catalogState != null && catalogState is! String) {
            throw const FormatException("\"catalog_state\" must be a string.");
        }

        return Diagnostics(
            contractVersion: contractVersion,
            agentVersion: _requireString(json, "agent_version"),
            os: _requireString(json, "os"),
            architecture: _requireString(json, "architecture"),
            windows: windows == null ? null : WindowsVersion.fromJson(windows),
            installedPatches: installedPatches,
            catalogState: catalogState as String? ?? "unknown",
        );
    }

    /// Version of the JSON contract used by this payload.
    final ContractVersion contractVersion;

    /// Version of the running agent build.
    final String agentVersion;

    /// Operating system identifier reported by the Rust core.
    final String os;

    /// CPU architecture identifier reported by the Rust core.
    final String architecture;

    /// Windows version data, absent on non-Windows hosts.
    final WindowsVersion? windows;

    /// Installed Windows updates reported without elevation.
    final List<String> installedPatches;

    /// State of the compatibility catalog for the detected build.
    final String catalogState;
}

/// Windows version block read from the registry by the Rust core.
class WindowsVersion {
    const WindowsVersion({
        required this.productName,
        required this.displayVersion,
        required this.build,
        this.updateBuildRevision,
    });

    /// Parses the Windows block of the diagnostics payload.
    factory WindowsVersion.fromJson(Map<String, dynamic> json) {
        final revision = json["update_build_revision"];

        if (revision != null && revision is! int) {
            throw const FormatException("\"update_build_revision\" must be an integer.");
        }

        return WindowsVersion(
            productName: _requireString(json, "product_name"),
            displayVersion: _requireString(json, "display_version"),
            build: _requireString(json, "build"),
            updateBuildRevision: revision,
        );
    }

    /// Marketing product name, for example `Windows 10 Home`.
    final String productName;

    /// Feature update version, for example `22H2`.
    final String displayVersion;

    /// Build number, for example `19045`.
    final String build;

    /// Update Build Revision (UBR), for example `6466`.
    final int? updateBuildRevision;
}

String _requireString(Map<String, dynamic> json, String key) {
    final value = json[key];

    if (value is String && value.isNotEmpty) {
        return value;
    }

    throw FormatException("Missing or invalid \"$key\" in the diagnostics payload.");
}
