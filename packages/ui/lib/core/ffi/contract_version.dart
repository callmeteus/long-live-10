/// Version of the JSON contract shared with the Rust core.
class ContractVersion {
    const ContractVersion({
        required this.major,
        required this.minor,
        required this.patch,
    });

    /// Contract version written by the current Rust producers.
    const ContractVersion.current()
            : major = supportedMajor,
                minor = 0,
                patch = 0;

    /// Major version understood by this build of the interface.
    static const int supportedMajor = 1;

    /// Parses a contract version object.
    factory ContractVersion.fromJson(Object? value) {
        if (value is! Map<String, dynamic>) {
            throw const FormatException(
                "\"contract_version\" must be an object.",
            );
        }

        return ContractVersion(
            major: _requireInt(value, "major"),
            minor: _requireInt(value, "minor"),
            patch: _requireInt(value, "patch"),
        );
    }

    /// Breaking contract generation.
    final int major;

    /// Backward compatible additions.
    final int minor;

    /// Corrections that do not change the shape.
    final int patch;

    /// Throws when this payload cannot be read safely.
    void ensureSupported() {
        if (major != supportedMajor) {
            throw FormatException(
                "Unsupported contract version $major, expected $supportedMajor.",
            );
        }
    }

    @override
    String toString() => "$major.$minor.$patch";
}

int _requireInt(Map<String, dynamic> json, String key) {
    final value = json[key];

    if (value is int && value >= 0) {
        return value;
    }

    throw FormatException("Missing or invalid \"$key\" in the contract version.");
}
