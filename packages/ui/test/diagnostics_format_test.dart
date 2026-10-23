import "package:flutter_test/flutter_test.dart";
import "package:long_live_10/modules/diagnostics/diagnostics_format.dart";

void main() {
    group("formatOperatingSystem", () {
        test("maps the windows identifier to a product label", () {
            expect(formatOperatingSystem("windows"), "Windows");
            expect(formatOperatingSystem("WINDOWS"), "Windows");
        });

        test("maps other known identifiers", () {
            expect(formatOperatingSystem("linux"), "Linux");
            expect(formatOperatingSystem("macos"), "macOS");
            expect(formatOperatingSystem("darwin"), "macOS");
        });

        test("capitalizes unknown identifiers", () {
            expect(formatOperatingSystem("freebsd"), "Freebsd");
        });

        test("keeps an empty identifier unchanged", () {
            expect(formatOperatingSystem(""), "");
        });
    });
}
