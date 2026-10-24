import "package:flutter_test/flutter_test.dart";
import "package:long_live_10/core/ffi/agent_client.dart";
import "package:long_live_10/core/ffi/diagnostics_models.dart";

class _FakeClient implements AgentClient {
    _FakeClient(this._results);

    final List<Object> _results;
    int _index = 0;

    @override
    Future<Diagnostics> readDiagnostics() async {
        final result = _results[_index++];
        if (result is Exception) {
            throw result;
        }
        return result as Diagnostics;
    }
}

Diagnostics _diagnostics(String version) {
    return Diagnostics(
        agentVersion: version,
        os: "windows",
        architecture: "x86_64",
    );
}

void main() {
    group("CachedAgentClient", () {
        test("returns and caches a successful read", () async {
            final client = CachedAgentClient(
                _FakeClient([_diagnostics("0.1.0")]),
            );

            final result = await client.readDiagnostics();

            expect(result.agentVersion, "0.1.0");
            expect(client.hasCachedDiagnostics, isTrue);
        });

        test("returns the cached value after a later failure", () async {
            final client = CachedAgentClient(
                _FakeClient([
                    _diagnostics("0.1.0"),
                    Exception("offline"),
                ]),
            );

            await client.readDiagnostics();
            final result = await client.readDiagnostics();

            expect(result.agentVersion, "0.1.0");
        });

        test("rethrows when there is no cached value", () async {
            final client = CachedAgentClient(
                _FakeClient([Exception("offline")]),
            );

            expect(() => client.readDiagnostics(), throwsException);
            expect(client.hasCachedDiagnostics, isFalse);
        });
    });
}
