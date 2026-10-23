import "dart:convert";
import "dart:io";

import "package:flutter_test/flutter_test.dart";
import "package:long_live_10/core/ffi/diagnostics_models.dart";
import "package:long_live_10/modules/catalog/catalog_models.dart";

Map<String, dynamic> _readFixture(String name) {
    final file = File("../ui-contract/fixtures/$name");
    final decoded = jsonDecode(file.readAsStringSync());

    if (decoded is! Map<String, dynamic>) {
        throw FormatException("Fixture $name is not a JSON object.");
    }

    return decoded;
}

void main() {
    group("Rust UI contract fixtures", () {
        test("diagnostics fixture matches the Dart model", () {
            final diagnostics = Diagnostics.fromJson(
                _readFixture("diagnostics.json"),
            );

            expect(diagnostics.contractVersion.major, 1);
            expect(diagnostics.agentVersion, "0.1.0");
            expect(diagnostics.os, "windows");
            expect(diagnostics.architecture, "x86_64");
            expect(diagnostics.windows?.productName, "Windows 10 Pro");
            expect(diagnostics.windows?.displayVersion, "22H2");
            expect(diagnostics.windows?.build, "19045");
            expect(diagnostics.windows?.updateBuildRevision, 6466);
        });

        test("catalog fixture matches the Dart model", () {
            final catalog = CatalogPayload.fromJson(
                _readFixture("catalog.json"),
            );

            expect(catalog.contractVersion.major, 1);
            expect(catalog.targetBuilds, hasLength(1));
            expect(catalog.patches, hasLength(1));
            expect(catalog.compatibility, hasLength(2));

            final build = catalog.targetBuilds[0];
            expect(build.id, "win10-22h2-x64");
            expect(build.productName, "Windows 10 Pro");
            expect(build.build, 19045);
            expect(build.ubr, 6466);

            final patch = catalog.patches[0];
            expect(patch.id, "kb-test-0001");
            expect(patch.cves, ["CVE-2026-0001"]);
            expect(patch.status, "experimental");

            final software = catalog.compatibility[0];
            expect(software.id, "software-dotnet-framework-4-8-1");
            expect(software.kind, "software");
            expect(software.version, "4.8.1");
            expect(software.driverVersion, isNull);

            final driver = catalog.compatibility[1];
            expect(driver.id, "driver-example-network");
            expect(driver.kind, "driver");
            expect(driver.hardwareIds, ["PCI\\VEN_1234&DEV_5678"]);
            expect(driver.driverVersion, "1.0.0.0");
        });

        test("rejects a catalog with an unsupported contract version", () {
            final json = _readFixture("catalog.json");
            json["contract_version"] = {
                "major": 2,
                "minor": 0,
                "patch": 0,
            };

            expect(
                () => CatalogPayload.fromJson(json),
                throwsFormatException,
            );
        });

        test("rejects a catalog without a target build array", () {
            final json = _readFixture("catalog.json");
            json.remove("target_builds");

            expect(
                () => CatalogPayload.fromJson(json),
                throwsFormatException,
            );
        });

        test("rejects a patch with an empty display title", () {
            final json = _readFixture("catalog.json");
            final patches = json["patches"] as List<dynamic>;
            final patch = patches[0] as Map<String, dynamic>;
            patch["title"] = "";

            expect(
                () => CatalogPayload.fromJson(json),
                throwsFormatException,
            );
        });
    });
}
