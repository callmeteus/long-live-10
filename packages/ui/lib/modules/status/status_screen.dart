import "package:flutter/material.dart";

import "../../core/ffi/agent_client.dart";
import "../../core/ffi/diagnostics_models.dart";
import "../../core/i18n/app_localizations.dart";
import "../../core/i18n/app_strings.dart";
import "system_status.dart";

/// Support summary for the PC that runs the agent.
class StatusScreen extends StatefulWidget {
    const StatusScreen({super.key, required this.client});

    /// Source of the diagnostics used to derive the status.
    final AgentClient client;

    @override
    State<StatusScreen> createState() => _StatusScreenState();
}

class _StatusScreenState extends State<StatusScreen> {
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
                    return _LoadingState(strings: strings.status);
                }

                if (snapshot.hasError) {
                    return _ErrorState(
                        strings: strings.status,
                        onRetry: _reload,
                    );
                }

                return _StatusView(
                    status: SystemStatus.fromDiagnostics(snapshot.data!),
                    strings: strings.status,
                    onRefresh: _reload,
                );
            },
        );
    }
}

class _StatusView extends StatelessWidget {
    const _StatusView({
        required this.status,
        required this.strings,
        required this.onRefresh,
    });

    final SystemStatus status;
    final StatusStrings strings;
    final VoidCallback onRefresh;

    @override
    Widget build(BuildContext context) {
        final windows = status.windows;

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
                            _SupportCard(
                                level: status.level,
                                title: _supportTitle(status.level),
                                body: _supportBody(status.level),
                            ),
                            const SizedBox(height: 16),
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
                                                    label: strings.currentBuild,
                                                    value: windows == null
                                                            ? strings.notReported
                                                            : _buildLabel(windows),
                                                ),
                                            ),
                                            SizedBox(
                                                width: width,
                                                child: _InfoCard(
                                                    icon: Icons.developer_board,
                                                    label: strings.architecture,
                                                    value: status.architecture ?? strings.notReported,
                                                ),
                                            ),
                                            SizedBox(
                                                width: width,
                                                child: _InfoCard(
                                                    icon: Icons.memory,
                                                    label: strings.agentVersion,
                                                    value: status.agentVersion ?? strings.notReported,
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

    String _supportTitle(SupportLevel level) {
        switch (level) {
            case SupportLevel.supported:
                return strings.supportedTitle;
            case SupportLevel.unsupported:
                return strings.unsupportedTitle;
            case SupportLevel.unknown:
                return strings.unknownTitle;
        }
    }

    String _supportBody(SupportLevel level) {
        switch (level) {
            case SupportLevel.supported:
                return strings.supportedBody;
            case SupportLevel.unsupported:
                return strings.unsupportedBody;
            case SupportLevel.unknown:
                return strings.unknownBody;
        }
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

    final StatusStrings strings;
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

class _SupportCard extends StatelessWidget {
    const _SupportCard({
        required this.level,
        required this.title,
        required this.body,
    });

    final SupportLevel level;
    final String title;
    final String body;

    @override
    Widget build(BuildContext context) {
        final colors = Theme.of(context).colorScheme;

        final (IconData icon, Color color) = switch (level) {
            SupportLevel.supported => (Icons.verified, colors.primary),
            SupportLevel.unsupported => (Icons.block, colors.error),
            SupportLevel.unknown => (Icons.help_outline, colors.onSurfaceVariant),
        };

        return Card(
            child: Padding(
                padding: const EdgeInsets.all(24),
                child: Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                        Icon(icon, size: 36, color: color),
                        const SizedBox(width: 20),
                        Expanded(
                            child: Column(
                                crossAxisAlignment: CrossAxisAlignment.start,
                                children: [
                                    Text(
                                        title,
                                        style: Theme.of(context).textTheme.titleLarge,
                                    ),
                                    const SizedBox(height: 8),
                                    Text(
                                        body,
                                        style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                                                    color: colors.onSurfaceVariant,
                                                ),
                                    ),
                                ],
                            ),
                        ),
                    ],
                ),
            ),
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

    final StatusStrings strings;

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

    final StatusStrings strings;
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
