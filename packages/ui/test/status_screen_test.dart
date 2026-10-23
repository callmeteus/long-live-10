import "package:flutter/material.dart";
import "package:flutter_localizations/flutter_localizations.dart";
import "package:flutter_test/flutter_test.dart";
import "package:long_live_10/core/ffi/agent_client.dart";
import "package:long_live_10/core/ffi/diagnostics_models.dart";
import "package:long_live_10/core/i18n/app_localizations.dart";
import "package:long_live_10/modules/status/status_screen.dart";

const _supported = Diagnostics(
    agentVersion: "0.1.0",
    os: "windows",
    architecture: "x86_64",
    windows: WindowsVersion(
        productName: "Windows 10 Home",
        displayVersion: "22H2",
        build: "19045",
        updateBuildRevision: 6466,
    ),
);

class _StubClient implements AgentClient {
    _StubClient(this._handler);

    final Future<Diagnostics> Function() _handler;

    @override
    Future<Diagnostics> readDiagnostics() => _handler();
}

Widget _buildApp(
    AgentClient client, {
    Locale locale = const Locale("en", "US"),
}) {
    return MaterialApp(
        locale: locale,
        localizationsDelegates: const [
            AppLocalizations.delegate,
            GlobalMaterialLocalizations.delegate,
            GlobalWidgetsLocalizations.delegate,
            GlobalCupertinoLocalizations.delegate,
        ],
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(body: StatusScreen(client: client)),
    );
}

void main() {
    testWidgets("shows a supported PC", (tester) async {
        await tester.pumpWidget(
            _buildApp(_StubClient(() async => _supported)),
        );
        await tester.pumpAndSettle();

        expect(find.text("System status"), findsOneWidget);
        expect(find.text("This PC is supported"), findsOneWidget);
        expect(find.text("Windows 10 Home 22H2"), findsOneWidget);
        expect(find.text("19045 (UBR 6466)"), findsOneWidget);
        expect(find.text("x86_64"), findsOneWidget);
        expect(find.text("0.1.0"), findsOneWidget);
    });

    testWidgets("shows an unsupported PC", (tester) async {
        const diagnostics = Diagnostics(
            agentVersion: "0.1.0",
            os: "windows",
            architecture: "x86_64",
            windows: WindowsVersion(
                productName: "Windows 10 Home",
                displayVersion: "21H2",
                build: "19044",
            ),
        );

        await tester.pumpWidget(
            _buildApp(_StubClient(() async => diagnostics)),
        );
        await tester.pumpAndSettle();

        expect(find.text("This PC is not supported"), findsOneWidget);
        expect(
            find.text(
                "This release covers Windows 10 22H2 on 64-bit. Nothing will be changed on this PC.",
            ),
            findsOneWidget,
        );
    });

    testWidgets("shows an unknown status without a Windows block", (tester) async {
        const diagnostics = Diagnostics(
            agentVersion: "0.1.0",
            os: "linux",
            architecture: "x86_64",
        );

        await tester.pumpWidget(
            _buildApp(_StubClient(() async => diagnostics)),
        );
        await tester.pumpAndSettle();

        expect(find.text("Status unavailable"), findsOneWidget);
        expect(find.text("Not reported"), findsNWidgets(2));
    });

    testWidgets("shows an error and recovers after retry", (tester) async {
        var calls = 0;
        final client = _StubClient(() async {
            calls++;

            if (calls == 1) {
                throw const AgentFfiException("library not found");
            }

            return _supported;
        });

        await tester.pumpWidget(_buildApp(client));
        await tester.pumpAndSettle();

        expect(find.text("Could not check this PC"), findsOneWidget);

        await tester.tap(find.text("Try again"));
        await tester.pumpAndSettle();

        expect(find.text("Could not check this PC"), findsNothing);
        expect(find.text("This PC is supported"), findsOneWidget);
        expect(calls, 2);
    });

    testWidgets("uses Portuguese strings for the pt-BR locale", (tester) async {
        await tester.pumpWidget(
            _buildApp(
                _StubClient(() async => _supported),
                locale: const Locale("pt", "BR"),
            ),
        );
        await tester.pumpAndSettle();

        expect(find.text("Status do sistema"), findsOneWidget);
        expect(find.text("Este PC é compatível"), findsOneWidget);
        expect(find.text("Atualizar"), findsOneWidget);
        expect(find.text("Compilação atual"), findsOneWidget);
    });
}
