import "package:flutter/material.dart";
import "package:flutter_localizations/flutter_localizations.dart";
import "package:flutter_test/flutter_test.dart";
import "package:long_live_10/core/ffi/agent_client.dart";
import "package:long_live_10/core/ffi/diagnostics_models.dart";
import "package:long_live_10/core/i18n/app_localizations.dart";
import "package:long_live_10/modules/diagnostics/diagnostics_screen.dart";

const _sample = Diagnostics(
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
        home: DiagnosticsScreen(client: client),
    );
}

void main() {
    testWidgets("renders the diagnostics returned by the client", (tester) async {
        await tester.pumpWidget(
            _buildApp(_StubClient(() async => _sample)),
        );
        await tester.pumpAndSettle();

        expect(find.text("System diagnostics"), findsOneWidget);
        expect(find.text("0.1.0"), findsOneWidget);
        expect(find.text("Windows"), findsOneWidget);
        expect(find.text("windows"), findsNothing);
        expect(find.text("x86_64"), findsOneWidget);
        expect(find.text("Windows 10 Home 22H2"), findsOneWidget);
        expect(find.text("19045 (UBR 6466)"), findsOneWidget);
    });

    testWidgets("uses Portuguese strings for the pt-BR locale", (tester) async {
        await tester.pumpWidget(
            _buildApp(
                _StubClient(() async => _sample),
                locale: const Locale("pt", "BR"),
            ),
        );
        await tester.pumpAndSettle();

        expect(find.text("Diagnóstico do sistema"), findsOneWidget);
        expect(find.text("Atualizar"), findsOneWidget);
        expect(find.text("Compilação do Windows"), findsOneWidget);
    });

    testWidgets("shows an error and recovers after retry", (tester) async {
        var calls = 0;
        final client = _StubClient(() async {
            calls++;

            if (calls == 1) {
                throw const AgentFfiException("library not found");
            }

            return _sample;
        });

        await tester.pumpWidget(_buildApp(client));
        await tester.pumpAndSettle();

        expect(find.text("Could not read diagnostics"), findsOneWidget);

        await tester.tap(find.text("Try again"));
        await tester.pumpAndSettle();

        expect(find.text("Could not read diagnostics"), findsNothing);
        expect(find.text("0.1.0"), findsOneWidget);
        expect(calls, 2);
    });

    testWidgets("shows not reported values on non-Windows hosts", (tester) async {
        const diagnostics = Diagnostics(
            agentVersion: "0.1.0",
            os: "linux",
            architecture: "x86_64",
        );

        await tester.pumpWidget(
            _buildApp(_StubClient(() async => diagnostics)),
        );
        await tester.pumpAndSettle();

        expect(find.text("Linux"), findsOneWidget);
        expect(find.text("Not reported"), findsNWidgets(2));
    });
}
