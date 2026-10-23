import "../../core/ffi/diagnostics_models.dart";

/// Support level derived from the reported Windows build.
enum SupportLevel {
    /// The build is covered by this release.
    supported,
    /// The build is known but not covered.
    unsupported,
    /// The build could not be determined safely.
    unknown,
}

/// Typed support summary shown by the status screen.
class SystemStatus {
    const SystemStatus({
        required this.level,
        this.windows,
        this.architecture,
        this.agentVersion,
    });

    /// Derives the support level from diagnostics produced by the Rust core.
    factory SystemStatus.fromDiagnostics(Diagnostics diagnostics) {
        final windows = diagnostics.windows;

        if (windows == null) {
            return SystemStatus(
                level: SupportLevel.unknown,
                architecture: diagnostics.architecture,
                agentVersion: diagnostics.agentVersion,
            );
        }

        final build = int.tryParse(windows.build);

        if (build == null) {
            return SystemStatus(
                level: SupportLevel.unknown,
                windows: windows,
                architecture: diagnostics.architecture,
                agentVersion: diagnostics.agentVersion,
            );
        }

        final isWindows10 = windows.productName
                .toLowerCase()
                .contains("windows 10");
        final isTargetVersion = windows.displayVersion == "22H2";
        final isTargetBuild = build == 19045;
        final isTargetArchitecture = diagnostics.architecture == "x86_64";
        final supported = isWindows10 &&
                isTargetVersion &&
                isTargetBuild &&
                isTargetArchitecture;

        return SystemStatus(
            level: supported ? SupportLevel.supported : SupportLevel.unsupported,
            windows: windows,
            architecture: diagnostics.architecture,
            agentVersion: diagnostics.agentVersion,
        );
    }

    /// Support level for the reported build.
    final SupportLevel level;

    /// Windows version block, when it could be read.
    final WindowsVersion? windows;

    /// CPU architecture reported by the agent.
    final String? architecture;

    /// Version of the running agent build.
    final String? agentVersion;
}
