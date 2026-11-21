import "dart:convert";
import "dart:io";

import "package:flutter/material.dart";

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

  /// Runs one `ll10-agent patches` command and records its outcome.
  ///
  /// All callers used to repeat the same Process.run / setState / "Error: $e"
  /// block. Keeping the error card in one place makes the visible wording
  /// consistent and avoids a busy flag left set when a command throws.
  Future<ProcessResult?> _runAgent(List<String> arguments, {required bool busy}) async {
    setState(() => busy ? _busy = true : _loading = true);
    try {
      final result = await Process.run("ll10-agent", arguments);
      if (result.exitCode != 0) {
        _status = "Failed: ${result.stderr}";
      }
      return result;
    } catch (e) {
      _status = "Error: $e";
      return null;
    } finally {
      setState(() {
        _loading = false;
        _busy = false;
      });
    }
  }

  Future<void> _loadPatches() async {
    final result = await _runAgent(["patches", "list", "--json"], busy: false);
    if (result == null || result.exitCode != 0) return;
    final data = jsonDecode(result.stdout as String) as List<dynamic>;
    setState(() {
      _patches = data.map((e) => e.toString()).toList();
    });
  }

  Future<void> _showPatch(String id) async {
    final result = await _runAgent(["patches", "show", id, "--json"], busy: true);
    if (result == null || result.exitCode != 0) return;
    setState(() {
      _selected = id;
      _detail = jsonDecode(result.stdout as String) as Map<String, dynamic>;
    });
  }

  Future<void> _verify() async {
    if (_selected == null) return;
    final result = await _runAgent(["patches", "verify", _selected!, "--json"], busy: true);
    if (result == null || result.exitCode != 0) return;
    setState(() => _status = result.stdout as String);
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
    final result = await _runAgent(["patches", "apply", _selected!, "--confirm", "--json"], busy: true);
    if (result == null || result.exitCode != 0) return;
    setState(() => _status = result.stdout as String);
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
    final result = await _runAgent(["patches", "revert", _selected!, "--confirm", "--json"], busy: true);
    if (result == null || result.exitCode != 0) return;
    setState(() => _status = result.stdout as String);
  }

  @override
  Widget build(BuildContext context) {
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