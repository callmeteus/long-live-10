import "dart:convert";
import "dart:ffi";
import "dart:io";

import "package:ffi/ffi.dart";

import "diagnostics_models.dart";

/// Read-only diagnostics source used by the interface.
abstract interface class AgentClient {
    /// Reads the current diagnostics from the Rust core.
    Future<Diagnostics> readDiagnostics();
}

/// Resolves an [AgentClient] on first use so a missing library becomes an
/// error state instead of a startup crash.
class DeferredAgentClient implements AgentClient {
    DeferredAgentClient(this._factory);

    final AgentClient Function() _factory;
    AgentClient? _client;

    @override
    Future<Diagnostics> readDiagnostics() async {
        final client = _client ??= _factory();
        return client.readDiagnostics();
    }
}

/// Failure raised at the Dart and Rust boundary.
class AgentFfiException implements Exception {
    const AgentFfiException(this.message);

    /// Human-readable cause, safe to log.
    final String message;

    @override
    String toString() => "AgentFfiException: $message";
}

/// Caches the last successful diagnostics so callers can keep working when
/// the agent is temporarily unavailable.
class CachedAgentClient implements AgentClient {
    CachedAgentClient(this._delegate);

    final AgentClient _delegate;

    Diagnostics? _cached;

    /// Whether a previous successful read is available.
    bool get hasCachedDiagnostics => _cached != null;

    @override
    Future<Diagnostics> readDiagnostics() async {
        try {
            final diagnostics = await _delegate.readDiagnostics();
            _cached = diagnostics;
            return diagnostics;
        } catch (error) {
            if (_cached != null) {
                return _cached!;
            }
            rethrow;
        }
    }
}

/// Loads `agent_ffi.dll` and reads diagnostics through the C ABI.
class FfiAgentClient implements AgentClient {
    FfiAgentClient._(this._diagnosticsJson, this._freeString, this.abiVersion);

    /// ABI version this client understands.
    static const int supportedAbiVersion = 1;

    /// Environment variable used to point at a specific library build.
    static const String libraryPathVariable = "LL10_AGENT_FFI";

    /// Default library name searched by the platform loader.
    static const String defaultLibraryName = "agent_ffi.dll";

    final Pointer<Utf8> Function() _diagnosticsJson;
    final void Function(Pointer<Utf8>) _freeString;

    /// ABI version reported by the loaded library.
    final int abiVersion;

    /// Loads the library and validates the ABI version.
    factory FfiAgentClient.load({String? libraryPath}) {
        final path = libraryPath ??
                Platform.environment[libraryPathVariable] ??
                defaultLibraryName;

        final library = DynamicLibrary.open(path);

        final abiVersion = library.lookupFunction<Uint32 Function(), int Function()>(
            "ll10_abi_version",
        )();

        if (abiVersion != supportedAbiVersion) {
            throw AgentFfiException(
                "Unsupported agent ABI $abiVersion, expected $supportedAbiVersion.",
            );
        }

        final diagnosticsJson = library.lookupFunction<
                Pointer<Utf8> Function(),
                Pointer<Utf8> Function()>("ll10_diagnostics_json");

        final freeString = library.lookupFunction<
                Void Function(Pointer<Utf8>),
                void Function(Pointer<Utf8>)>("ll10_free_string");

        return FfiAgentClient._(diagnosticsJson, freeString, abiVersion);
    }

    @override
    Future<Diagnostics> readDiagnostics() async {
        final pointer = _diagnosticsJson();

        if (pointer == nullptr) {
            throw const AgentFfiException(
                "The agent returned an empty diagnostics payload.",
            );
        }

        try {
            final json = pointer.toDartString();
            final decoded = jsonDecode(json);

            if (decoded is! Map<String, dynamic>) {
                throw const FormatException(
                    "The diagnostics payload is not a JSON object.",
                );
            }

            return Diagnostics.fromJson(decoded);
        } finally {
            _freeString(pointer);
        }
    }
}
