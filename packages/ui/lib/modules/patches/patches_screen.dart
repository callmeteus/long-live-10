import "dart:convert";
import "dart:io";

import "package:flutter/material.dart";

import "../../core/i18n/app_localizations.dart";

/// Patches management screen.
class PatchesScreen extends StatefulWidget {
  const PatchesScreen({super.key});

  @override
  State<PatchesScreen> createState() => _PatchesScreenState();
}

class _PatchesScreenState extends State<PatchesScreen> {
  List<String> _patches = [];
  String? _selected;
  Map<String, dynamic>? _detail;
  String? _status;
  bool _loading = true;
  bool _busy = false;

  @override
  void initState() {
    super.initState();
    _loadPatches();
  }

  Future<void> _loadPatches() async {
    setState(() => _loading = true);
    try {
      final result = await Process.run("ll10-agent", ["patches", "list", "--json"]);
      if (result.exitCode == 0) {
        final data = jsonDecode(result.stdout as String) as List<dynamic>;
        setState(() {
          _patches = data.map((e) => e.toString()).toList();
          _loading = false;
        });
      } else {
        setState(() {
          _status = "Failed to list patches: ${result.stderr}";
          _loading = false;
        });
      }
    } catch (e) {
      setState(() {
        _status = "Error: $e";
        _loading = false;
      });
    }
  }

  Future<void> _showPatch(String id) async {
    setState(() => _busy = true);
    try {
      final result = await Process.run("ll10-agent", ["patches", "show", id, "--json"]);
      if (result.exitCode == 0) {
        setState(() {
          _selected = id;
          _detail = jsonDecode(result.stdout as String) as Map<String, dynamic>;
          _busy = false;
        });
      } else {
        setState(() {
          _status = "Failed: ${result.stderr}";
          _busy = false;
        });
      }
    } catch (e) {
      setState(() {
        _status = "Error: $e";
        _busy = false;
      });
    }
  }

  Future<void> _verify() async {
    if (_selected == null) return;
    setState(() => _busy = true);
    try {
      final result = await Process.run("ll10-agent", ["patches", "verify", _selected!, "--json"]);
      setState(() {
        _status = result.stdout as String;
        _busy = false;
      });
    } catch (e) {
      setState(() {
        _status = "Error: $e";
        _busy = false;
      });
    }
  }

  Future<void> _apply() async {
    if (_selected == null) return;
    final confirm = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text("Confirm apply"),
        content: Text("Apply patch $_selected?"),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx, false), child: const Text("Cancel")),
          FilledButton(onPressed: () => Navigator.pop(ctx, true), child: const Text("Apply")),
        ],
      ),
    );
    if (confirm != true) return;
    setState(() => _busy = true);
    try {
      final result = await Process.run("ll10-agent", ["patches", "apply", _selected!, "--confirm", "--json"]);
      setState(() {
        _status = result.stdout as String;
        _busy = false;
      });
    } catch (e) {
      setState(() {
        _status = "Error: $e";
        _busy = false;
      });
    }
  }

  Future<void> _revert() async {
    if (_selected == null) return;
    final confirm = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text("Confirm revert"),
        content: Text("Revert patch $_selected?"),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx, false), child: const Text("Cancel")),
          FilledButton(onPressed: () => Navigator.pop(ctx, true), child: const Text("Revert")),
        ],
      ),
    );
    if (confirm != true) return;
    setState(() => _busy = true);
    try {
      final result = await Process.run("ll10-agent", ["patches", "revert", _selected!, "--confirm", "--json"]);
      setState(() {
        _status = result.stdout as String;
        _busy = false;
      });
    } catch (e) {
      setState(() {
        _status = "Error: $e";
        _busy = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final strings = AppLocalizations.of(context).strings;
    return Row(
      children: [
        SizedBox(
          width: 240,
          child: _loading
              ? const Center(child: CircularProgressIndicator())
              : ListView.builder(
                  itemCount: _patches.length,
                  itemBuilder: (ctx, i) {
                    final id = _patches[i];
                    return ListTile(
                      title: Text(id),
                      selected: id == _selected,
                      onTap: () => _showPatch(id),
                    );
                  },
                ),
        ),
        const VerticalDivider(width: 1),
        Expanded(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: _busy
                ? const Center(child: CircularProgressIndicator())
                : Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      if (_detail != null) ...[
                        Text(_detail!["title"]?.toString() ?? "", style: Theme.of(context).textTheme.titleLarge),
                        const SizedBox(height: 8),
                        Text("Severity: ${_detail!["severity"]}"),
                        Text("Target: ${_detail!["target"]}"),
                        const SizedBox(height: 16),
                        Row(
                          children: [
                            FilledButton(onPressed: _verify, child: const Text("Verify")),
                            const SizedBox(width: 8),
                            FilledButton(onPressed: _apply, child: const Text("Apply")),
                            const SizedBox(width: 8),
                            OutlinedButton(onPressed: _revert, child: const Text("Revert")),
                          ],
                        ),
                      ],
                      if (_status != null) ...[
                        const SizedBox(height: 16),
                        Expanded(child: SingleChildScrollView(child: Text(_status!))),
                      ],
                    ],
                  ),
          ),
        ),
      ],
    );
  }
}