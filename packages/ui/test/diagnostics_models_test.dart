import "package:flutter_test/flutter_test.dart";
import "package:long_live_10/core/ffi/diagnostics_models.dart";

Map<String, dynamic> _contract({
    int major = 1,
    int minor = 0,
    int patch = 0,
}) {
    return {
        "major": major,
        "minor": minor,
        "patch": patch,
    };
}

void main() {
    group("Diagnostics.fromJson", () {
        test("parses the full payload", () {
            final diagnostics = Diagnostics.fromJson({
                "contract_version": _contract(),
                "agent_version": "0.1.0",
                "os": "windows",
                "architecture": "x86_64",
                "windows": {
                    "product_name": "Windows 10 Home",
                    "display_version": "22H2",
                    "build": "19045",
                    "update_build_revision": 6466,
                },
            });

            expect(diagnostics.contractVersion.major, 1);
            expect(diagnostics.agentVersion, "0.1.0");
            expect(diagnostics.os, "windows");
            expect(diagnostics.architecture, "x86_64");
            expect(diagnostics.windows?.productName, "Windows 10 Home");
            expect(diagnostics.windows?.displayVersion, "22H2");
            expect(diagnostics.windows?.build, "19045");
            expect(diagnostics.windows?.updateBuildRevision, 6466);
        });

        test("accepts a missing windows block", () {
            final diagnostics = Diagnostics.fromJson({
                "contract_version": _contract(),
                "agent_version": "0.1.0",
                "os": "linux",
                "architecture": "x86_64",
                "windows": null,
            });

            expect(diagnostics.windows, isNull);
        });

        test("accepts a missing update revision", () {
            final windows = WindowsVersion.fromJson({
                "product_name": "Windows 10 Pro",
                "display_version": "22H2",
                "build": "19045",
            });

            expect(windows.updateBuildRevision, isNull);
        });

        test("rejects a payload without the agent version", () {
            expect(
                () => Diagnostics.fromJson({
                    "contract_version": _contract(),
                    "os": "windows",
                    "architecture": "x86_64",
                }),
                throwsFormatException,
            );
        });

        test("rejects an empty string value", () {
            expect(
                () => Diagnostics.fromJson({
                    "contract_version": _contract(),
                    "agent_version": "",
                    "os": "windows",
                    "architecture": "x86_64",
                }),
                throwsFormatException,
            );
        });

        test("rejects a non-integer update revision", () {
            expect(
                () => WindowsVersion.fromJson({
                    "product_name": "Windows 10 Pro",
                    "display_version": "22H2",
                    "build": "19045",
                    "update_build_revision": "6466",
                }),
                throwsFormatException,
            );
        });

        test("rejects a windows block that is not an object", () {
            expect(
                () => Diagnostics.fromJson({
                    "contract_version": _contract(),
                    "agent_version": "0.1.0",
                    "os": "windows",
                    "architecture": "x86_64",
                    "windows": "unsupported",
                }),
                throwsFormatException,
            );
        });

        test("rejects an unsupported contract major version", () {
            expect(
                () => Diagnostics.fromJson({
                    "contract_version": _contract(major: 2),
                    "agent_version": "0.1.0",
                    "os": "windows",
                    "architecture": "x86_64",
                }),
                throwsFormatException,
            );
        });

        test("rejects a payload without a contract version", () {
            expect(
                () => Diagnostics.fromJson({
                    "agent_version": "0.1.0",
                    "os": "windows",
                    "architecture": "x86_64",
                }),
                throwsFormatException,
            );
        });
    });
}
