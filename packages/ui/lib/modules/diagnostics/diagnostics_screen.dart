import "package:flutter/material.dart";

import "../../core/ffi/agent_client.dart";
import "../../core/ffi/diagnostics_models.dart";
import "../../core/i18n/app_localizations.dart";
import "../../core/i18n/app_strings.dart";
import "diagnostics_format.dart";

/// Read-only overview of the agent and the Windows build it detects.
class DiagnosticsScreen extends StatefulWidget {
    const DiagnosticsScreen({super.key, required this.client});

    /// Source of the diagnostics data.
    final AgentClient client;

    @override
    State<DiagnosticsScreen> createState() => _DiagnosticsScreenState();
}

class _DiagnosticsScreenState extends State<DiagnosticsScreen> {
    late Future<Diagnostics> _diagnostics;

    @override
    void initState() {
        super.initState();
        _diagnostics = widget.client.readDiagnostics();
    }

    void _reload() {
        setState(() {
            _diagnostics = widget.client.readDiagnostics();
        });
    }

    @override
    Widget build(BuildContext context) {
        final strings = AppLocalizations.of(context).strings;

        return FutureBuilder<Diagnostics>(
            future: _diagnostics,
            builder: (context, snapshot) {
                if (snapshot.connectionState != ConnectionState.done) {
                    return _LoadingState(strings: strings.diagnostics);
                }

                if (snapshot.hasError) {
                    return _ErrorState(
                        strings: strings.diagnostics,
                        onRetry: _reload,
                    );
                }

                return _DiagnosticsView(
                    diagnostics: snapshot.data!,
                    strings: strings.diagnostics,
                    onRefresh: _reload,
                );
            },
        );
    }
}

class _DiagnosticsView extends StatelessWidget {
    const _DiagnosticsView({
        required this.diagnostics,
        required this.strings,
        required this.onRefresh,
    });

    final Diagnostics diagnostics;
    final DiagnosticsStrings strings;
    final VoidCallback onRefresh;

    @override
    Widget build(BuildContext context) {
        final windows = diagnostics.windows;

        return SingleChildScrollView(
            padding: const EdgeInsets.all(24),
            child: Center(
                child: ConstrainedBox(
                    constraints: const BoxConstraints(maxWidth: 1040),
                    child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                            _Header(strings: strings, onRefresh: onRefresh),
                            const SizedBox(height: 24),
                            LayoutBuilder(
                                builder: (context, constraints) {
                                    final columns = constraints.maxWidth >= 880
                                            ? 3
                                            : constraints.maxWidth >= 560
                                                    ? 2
                                                    : 1;
                                    const spacing = 16.0;
                                    final width =
                                            (constraints.maxWidth - spacing * (columns - 1)) /
                                                    columns;

                                    return Wrap(
                                        spacing: spacing,
                                        runSpacing: spacing,
                                        children: [
                                            SizedBox(
                                                width: width,
                                                child: _InfoCard(
                                                    icon: Icons.memory,
                                                    label: strings.agentVersion,
                                                    value: diagnostics.agentVersion,
                                                ),
                                            ),
                                            SizedBox(
                                                width: width,
                                                child: _InfoCard(
                                                    icon: Icons.computer,
                                                    label: strings.operatingSystem,
                                                    value: formatOperatingSystem(diagnostics.os),
                                                ),
                                            ),
                                            SizedBox(
                                                width: width,
                                                child: _InfoCard(
                                                    icon: Icons.developer_board,
                                                    label: strings.architecture,
                                                    value: diagnostics.architecture,
                                                ),
                                            ),
                                            SizedBox(
                                                width: width,
                                                child: _InfoCard(
                                                    icon: Icons.window,
                                                    label: strings.windowsVersion,
                                                    value: windows == null
                                                            ? strings.notReported
                                                            : "${windows.productName} ${windows.displayVersion}",
                                                ),
                                            ),
                                            SizedBox(
                                                width: width,
                                                child: _InfoCard(
                                                    icon: Icons.tag,
                                                    label: strings.build,
                                                    value: windows == null
                                                            ? strings.notReported
                                                            : _buildLabel(windows),
                                                ),
                                            ),
                                        ],
                                    );
                                },
                            ),
                        ],
                    ),
                ),
            ),
        );
    }

    String _buildLabel(WindowsVersion windows) {
        final revision = windows.updateBuildRevision;

        if (revision == null) {
            return windows.build;
        }

        return "${windows.build} (UBR $revision)";
    }
}

class _Header extends StatelessWidget {
    const _Header({required this.strings, required this.onRefresh});

    final DiagnosticsStrings strings;
    final VoidCallback onRefresh;

    @override
    Widget build(BuildContext context) {
        final title = Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
                Text(
                    strings.title,
                    style: Theme.of(context).textTheme.headlineSmall,
                ),
                const SizedBox(height: 4),
                Text(
                    strings.subtitle,
                    style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                                color: Theme.of(context).colorScheme.onSurfaceVariant,
                            ),
                ),
            ],
        );

        final button = OutlinedButton.icon(
            onPressed: onRefresh,
            icon: const Icon(Icons.refresh),
            label: Text(strings.refresh),
        );

        return LayoutBuilder(
            builder: (context, constraints) {
                if (constraints.maxWidth < 520) {
                    return Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                            title,
                            const SizedBox(height: 12),
                            button,
                        ],
                    );
                }

                return Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                        Expanded(child: title),
                        const SizedBox(width: 16),
                        button,
                    ],
                );
            },
        );
    }
}

class _InfoCard extends StatelessWidget {
    const _InfoCard({
        required this.icon,
        required this.label,
        required this.value,
    });

    final IconData icon;
    final String label;
    final String value;

    @override
    Widget build(BuildContext context) {
        final colors = Theme.of(context).colorScheme;

        return Card(
            child: Padding(
                padding: const EdgeInsets.all(20),
                child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                        Row(
                            children: [
                                Icon(icon, size: 18, color: colors.primary),
                                const SizedBox(width: 8),
                                Expanded(
                                    child: Text(
                                        label,
                                        style: Theme.of(context).textTheme.labelLarge?.copyWith(
                                                    color: colors.onSurfaceVariant,
                                                ),
                                    ),
                                ),
                            ],
                        ),
                        const SizedBox(height: 12),
                        SelectableText(
                            value,
                            style: Theme.of(context).textTheme.titleMedium,
                        ),
                    ],
                ),
            ),
        );
    }
}

class _LoadingState extends StatelessWidget {
    const _LoadingState({required this.strings});

    final DiagnosticsStrings strings;

    @override
    Widget build(BuildContext context) {
        return Center(
            child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                    const CircularProgressIndicator(),
                    const SizedBox(height: 16),
                    Text(strings.loading),
                ],
            ),
        );
    }
}

class _ErrorState extends StatelessWidget {
    const _ErrorState({required this.strings, required this.onRetry});

    final DiagnosticsStrings strings;
    final VoidCallback onRetry;

    @override
    Widget build(BuildContext context) {
        return Center(
            child: ConstrainedBox(
                constraints: const BoxConstraints(maxWidth: 420),
                child: Padding(
                    padding: const EdgeInsets.all(24),
                    child: Column(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                            Icon(
                                Icons.error_outline,
                                size: 40,
                                color: Theme.of(context).colorScheme.error,
                            ),
                            const SizedBox(height: 16),
                            Text(
                                strings.errorTitle,
                                textAlign: TextAlign.center,
                                style: Theme.of(context).textTheme.titleLarge,
                            ),
                            const SizedBox(height: 8),
                            Text(
                                strings.errorHint,
                                textAlign: TextAlign.center,
                                style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                                            color: Theme.of(context).colorScheme.onSurfaceVariant,
                                        ),
                            ),
                            const SizedBox(height: 20),
                            FilledButton.icon(
                                onPressed: onRetry,
                                icon: const Icon(Icons.refresh),
                                label: Text(strings.retry),
                            ),
                        ],
                    ),
                ),
            ),
        );
    }
}
