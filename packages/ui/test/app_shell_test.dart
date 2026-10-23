import "package:flutter_test/flutter_test.dart";
import "package:long_live_10/core/ffi/agent_client.dart";
import "package:long_live_10/core/ffi/diagnostics_models.dart";
import "package:long_live_10/main.dart";

const _sample = Diagnostics(
    agentVersion: "0.1.0",
    os: "windows",
    architecture: "x86_64",
    windows: WindowsVersion(
        productName: "Windows 10 Pro",
        displayVersion: "22H2",
        build: "19045",
        updateBuildRevision: 6466,
    ),
);

class _StubClient implements AgentClient {
    @override
    Future<Diagnostics> readDiagnostics() async => _sample;
}

void main() {
    testWidgets("starts on status and switches to diagnostics", (tester) async {
        await tester.pumpWidget(LongLive10App(client: _StubClient()));
        await tester.pumpAndSettle();

        expect(find.text("System status"), findsOneWidget);
        expect(find.text("System diagnostics"), findsNothing);

        await tester.tap(find.text("Diagnostics"));
        await tester.pumpAndSettle();

        expect(find.text("System status"), findsNothing);
        expect(find.text("System diagnostics"), findsOneWidget);
    });

    testWidgets("switches back to status", (tester) async {
        await tester.pumpWidget(LongLive10App(client: _StubClient()));
        await tester.pumpAndSettle();

        await tester.tap(find.text("Diagnostics"));
        await tester.pumpAndSettle();

        await tester.tap(find.text("Status"));
        await tester.pumpAndSettle();

        expect(find.text("System status"), findsOneWidget);
        expect(find.text("System diagnostics"), findsNothing);
    });
}
