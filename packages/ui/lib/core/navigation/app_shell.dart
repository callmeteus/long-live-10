import "package:flutter/material.dart";

import "../ffi/agent_client.dart";
import "../i18n/app_localizations.dart";
import "../../modules/diagnostics/diagnostics_screen.dart";
import "../../modules/status/status_screen.dart";

/// Application shell with status and diagnostics pages.
class AppShell extends StatefulWidget {
    const AppShell({super.key, required this.client});

    /// Source used by both pages.
    final AgentClient client;

    @override
    State<AppShell> createState() => _AppShellState();
}

class _AppShellState extends State<AppShell> {
    int _index = 0;

    @override
    Widget build(BuildContext context) {
        final strings = AppLocalizations.of(context).strings;
        final pages = [
            StatusScreen(client: widget.client),
            DiagnosticsScreen(client: widget.client),
        ];

        final destinations = [
            _Destination(
                icon: Icons.shield_outlined,
                selectedIcon: Icons.shield,
                label: strings.common.statusTab,
            ),
            _Destination(
                icon: Icons.monitor_heart_outlined,
                selectedIcon: Icons.monitor_heart,
                label: strings.common.diagnosticsTab,
            ),
        ];

        final content = IndexedStack(index: _index, children: pages);
        final wide = MediaQuery.sizeOf(context).width >= 840;

        return Scaffold(
            appBar: AppBar(title: Text(strings.common.appTitle)),
            body: wide
                    ? Row(
                            children: [
                                NavigationRail(
                                    selectedIndex: _index,
                                    onDestinationSelected: _select,
                                    labelType: NavigationRailLabelType.all,
                                    destinations: destinations
                                            .map(
                                                (destination) => NavigationRailDestination(
                                                    icon: Icon(destination.icon),
                                                    selectedIcon: Icon(destination.selectedIcon),
                                                    label: Text(destination.label),
                                                ),
                                            )
                                            .toList(),
                                ),
                                const VerticalDivider(width: 1),
                                Expanded(child: content),
                            ],
                        )
                    : content,
            bottomNavigationBar: wide
                    ? null
                    : NavigationBar(
                            selectedIndex: _index,
                            onDestinationSelected: _select,
                            destinations: destinations
                                    .map(
                                        (destination) => NavigationDestination(
                                            icon: Icon(destination.icon),
                                            selectedIcon: Icon(destination.selectedIcon),
                                            label: destination.label,
                                        ),
                                    )
                                    .toList(),
                        ),
        );
    }

    void _select(int index) {
        setState(() {
            _index = index;
        });
    }
}

class _Destination {
    const _Destination({
        required this.icon,
        required this.selectedIcon,
        required this.label,
    });

    final IconData icon;
    final IconData selectedIcon;
    final String label;
}
