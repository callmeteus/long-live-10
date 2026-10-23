import "package:flutter_test/flutter_test.dart";
import "package:long_live_10/core/ffi/diagnostics_models.dart";
import "package:long_live_10/modules/status/system_status.dart";

Diagnostics _diagnostics({
    String productName = "Windows 10 Pro",
    String displayVersion = "22H2",
    String build = "19045",
    String architecture = "x86_64",
    int? revision = 6466,
}) {
    return Diagnostics(
        agentVersion: "0.1.0",
        os: "windows",
        architecture: architecture,
        windows: WindowsVersion(
            productName: productName,
            displayVersion: displayVersion,
            build: build,
            updateBuildRevision: revision,
        ),
    );
}

void main() {
    group("SystemStatus.fromDiagnostics", () {
        test("supports Windows 10 22H2 on 64-bit", () {
            final status = SystemStatus.fromDiagnostics(_diagnostics());

            expect(status.level, SupportLevel.supported);
            expect(status.windows?.build, "19045");
            expect(status.architecture, "x86_64");
        });

        test("rejects an older feature update", () {
            final status = SystemStatus.fromDiagnostics(
                _diagnostics(displayVersion: "21H2", build: "19044"),
            );

            expect(status.level, SupportLevel.unsupported);
        });

        test("rejects Windows 11", () {
            final status = SystemStatus.fromDiagnostics(
                _diagnostics(
                    productName: "Windows 11 Pro",
                    displayVersion: "23H2",
                    build: "22631",
                ),
            );

            expect(status.level, SupportLevel.unsupported);
        });

        test("rejects a 32-bit install", () {
            final status = SystemStatus.fromDiagnostics(
                _diagnostics(architecture: "x86"),
            );

            expect(status.level, SupportLevel.unsupported);
        });

        test("reports unknown without a Windows block", () {
            final status = SystemStatus.fromDiagnostics(
                const Diagnostics(
                    agentVersion: "0.1.0",
                    os: "linux",
                    architecture: "x86_64",
                ),
            );

            expect(status.level, SupportLevel.unknown);
            expect(status.windows, isNull);
        });

        test("reports unknown for a non-numeric build", () {
            final status = SystemStatus.fromDiagnostics(
                _diagnostics(build: "unknown"),
            );

            expect(status.level, SupportLevel.unknown);
        });
    });
}
