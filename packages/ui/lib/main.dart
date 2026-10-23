import "package:flutter/material.dart";
import "package:flutter_localizations/flutter_localizations.dart";

import "core/ffi/agent_client.dart";
import "core/i18n/app_localizations.dart";
import "core/navigation/app_shell.dart";

void main() {
    runApp(
        LongLive10App(
            client: DeferredAgentClient(FfiAgentClient.load),
        ),
    );
}

/// Application shell for the desktop interface.
class LongLive10App extends StatelessWidget {
    const LongLive10App({super.key, required this.client});

    /// Diagnostics source injected into the shell.
    final AgentClient client;

    @override
    Widget build(BuildContext context) {
        const seedColor = Color(0xFF1F6FEB);

        return MaterialApp(
            title: "Long Live 10",
            debugShowCheckedModeBanner: false,
            theme: ThemeData(
                colorScheme: ColorScheme.fromSeed(seedColor: seedColor),
                useMaterial3: true,
            ),
            localizationsDelegates: const [
                AppLocalizations.delegate,
                GlobalMaterialLocalizations.delegate,
                GlobalWidgetsLocalizations.delegate,
                GlobalCupertinoLocalizations.delegate,
            ],
            supportedLocales: AppLocalizations.supportedLocales,
            home: AppShell(client: client),
        );
    }
}
