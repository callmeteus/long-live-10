import "../../core/ffi/contract_version.dart";

/// Versioned catalog payload rendered by catalog and status screens.
class CatalogPayload {
    const CatalogPayload({
        required this.contractVersion,
        required this.targetBuilds,
        required this.patches,
        required this.compatibility,
    });

    /// Parses a catalog payload received from the Rust core.
    factory CatalogPayload.fromJson(Map<String, dynamic> json) {
        final contractVersion = ContractVersion.fromJson(
            json["contract_version"],
        );
        contractVersion.ensureSupported();

        return CatalogPayload(
            contractVersion: contractVersion,
            targetBuilds: _requireList(json["target_builds"], "target_builds")
                    .map((value) => CatalogTargetBuild.fromJson(
                                _requireObject(value, "target_builds[]"),
                            ))
                    .toList(),
            patches: _requireList(json["patches"], "patches")
                    .map((value) => CatalogPatch.fromJson(
                                _requireObject(value, "patches[]"),
                            ))
                    .toList(),
            compatibility: _requireList(
                json["compatibility"],
                "compatibility",
            )
                    .map((value) => CatalogCompatibilityEntry.fromJson(
                                _requireObject(value, "compatibility[]"),
                            ))
                    .toList(),
        );
    }

    /// Version of the JSON contract used by this payload.
    final ContractVersion contractVersion;

    /// Windows builds referenced by catalog entries.
    final List<CatalogTargetBuild> targetBuilds;

    /// Security patches available for the supported builds.
    final List<CatalogPatch> patches;

    /// Devices, drivers, and software assessed for the supported builds.
    final List<CatalogCompatibilityEntry> compatibility;
}

/// Windows build referenced by catalog entries.
class CatalogTargetBuild {
    const CatalogTargetBuild({
        required this.id,
        required this.productName,
        required this.displayVersion,
        required this.osVersion,
        required this.build,
        required this.ubr,
        required this.architecture,
    });

    /// Parses one target build.
    factory CatalogTargetBuild.fromJson(Map<String, dynamic> json) {
        return CatalogTargetBuild(
            id: _requireString(json, "id"),
            productName: _requireString(json, "product_name"),
            displayVersion: _requireString(json, "display_version"),
            osVersion: _requireString(json, "os_version"),
            build: _requireInt(json, "build"),
            ubr: _requireInt(json, "ubr"),
            architecture: _requireString(json, "architecture"),
        );
    }

    /// Stable build identifier such as `win10-22h2-x64`.
    final String id;

    /// Marketing product name.
    final String productName;

    /// Feature update version such as `22H2`.
    final String displayVersion;

    /// Kernel version such as `10.0`.
    final String osVersion;

    /// Windows build number.
    final int build;

    /// Update Build Revision.
    final int ubr;

    /// Target processor architecture.
    final String architecture;
}

/// Security patch summary shown in the catalog.
class CatalogPatch {
    const CatalogPatch({
        required this.id,
        required this.title,
        required this.description,
        required this.component,
        required this.cves,
        required this.builds,
        required this.architectures,
        required this.status,
    });

    /// Parses one patch entry.
    factory CatalogPatch.fromJson(Map<String, dynamic> json) {
        return CatalogPatch(
            id: _requireString(json, "id"),
            title: _requireString(json, "title"),
            description: _requireString(json, "description"),
            component: _requireString(json, "component"),
            cves: _requireStringList(json["cves"], "cves"),
            builds: _requireStringList(json["builds"], "builds"),
            architectures: _requireStringList(
                json["architectures"],
                "architectures",
            ),
            status: _requireString(json, "status"),
        );
    }

    /// Stable patch identifier.
    final String id;

    /// Short display title.
    final String title;

    /// Longer explanation of the fix.
    final String description;

    /// Windows component affected by the patch.
    final String component;

    /// CVE identifiers addressed by the patch.
    final List<String> cves;

    /// Target build numbers where the patch applies.
    final List<String> builds;

    /// Architectures supported by the patch.
    final List<String> architectures;

    /// Lifecycle state such as `experimental` or `validated`.
    final String status;
}

/// Device, driver, or software entry from the compatibility catalog.
class CatalogCompatibilityEntry {
    const CatalogCompatibilityEntry({
        required this.id,
        required this.kind,
        required this.name,
        required this.vendor,
        required this.description,
        required this.status,
        required this.testedBuilds,
        required this.architectures,
        required this.hardwareIds,
        required this.dependencies,
        required this.knownIssues,
        required this.limitations,
        this.version,
        this.driverVersion,
    });

    /// Parses one compatibility entry.
    factory CatalogCompatibilityEntry.fromJson(Map<String, dynamic> json) {
        return CatalogCompatibilityEntry(
            id: _requireString(json, "id"),
            kind: _requireString(json, "kind"),
            name: _requireString(json, "name"),
            vendor: _requireString(json, "vendor"),
            description: _requireString(json, "description"),
            status: _requireString(json, "status"),
            testedBuilds: _requireStringList(
                json["tested_builds"],
                "tested_builds",
            ),
            architectures: _requireStringList(
                json["architectures"],
                "architectures",
            ),
            hardwareIds: _optionalStringList(
                json["hardware_ids"],
                "hardware_ids",
            ),
            dependencies: _optionalStringList(
                json["dependencies"],
                "dependencies",
            ),
            knownIssues: _optionalStringList(
                json["known_issues"],
                "known_issues",
            ),
            limitations: _optionalStringList(
                json["limitations"],
                "limitations",
            ),
            version: _optionalString(json, "version"),
            driverVersion: _optionalString(json, "driver_version"),
        );
    }

    /// Stable entry identifier used by dependencies.
    final String id;

    /// Entry kind such as `software`, `device`, or `driver`.
    final String kind;

    /// Display name.
    final String name;

    /// Hardware or software vendor.
    final String vendor;

    /// What the entry covers.
    final String description;

    /// Lifecycle state such as `experimental` or `validated`.
    final String status;

    /// Target build identifiers used during assessment.
    final List<String> testedBuilds;

    /// Architectures covered by the assessment.
    final List<String> architectures;

    /// Hardware identifiers for devices and drivers.
    final List<String> hardwareIds;

    /// Entry identifiers that must be present first.
    final List<String> dependencies;

    /// Known defects or regressions.
    final List<String> knownIssues;

    /// Explicit limits of the assessment.
    final List<String> limitations;

    /// Software version under assessment.
    final String? version;

    /// Driver version under assessment.
    final String? driverVersion;
}

Map<String, dynamic> _requireObject(Object? value, String field) {
    if (value is Map<String, dynamic>) {
        return value;
    }

    throw FormatException("\"$field\" must be an object.");
}

List<dynamic> _requireList(Object? value, String field) {
    if (value is List<dynamic>) {
        return value;
    }

    throw FormatException("\"$field\" must be an array.");
}

String _requireString(Map<String, dynamic> json, String key) {
    final value = json[key];

    if (value is String && value.isNotEmpty) {
        return value;
    }

    throw FormatException("Missing or invalid \"$key\" in the catalog payload.");
}

String? _optionalString(Map<String, dynamic> json, String key) {
    final value = json[key];

    if (value == null) {
        return null;
    }

    if (value is String && value.isNotEmpty) {
        return value;
    }

    throw FormatException("Invalid \"$key\" in the catalog payload.");
}

int _requireInt(Map<String, dynamic> json, String key) {
    final value = json[key];

    if (value is int) {
        return value;
    }

    throw FormatException("Missing or invalid \"$key\" in the catalog payload.");
}

List<String> _requireStringList(Object? value, String field) {
    final list = _requireList(value, field);

    return list.map((item) {
        if (item is String && item.isNotEmpty) {
            return item;
        }

        throw FormatException("\"$field\" must contain non-empty strings.");
    }).toList();
}

List<String> _optionalStringList(Object? value, String field) {
    if (value == null) {
        return const [];
    }

    return _requireStringList(value, field);
}
