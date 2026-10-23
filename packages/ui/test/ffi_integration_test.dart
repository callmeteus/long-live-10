import "dart:io";

import "package:flutter_test/flutter_test.dart";
import "package:long_live_10/core/ffi/agent_client.dart";

void main() {
    final libraryPath = Platform.environment["LL10_AGENT_FFI"];
    final skipReason = libraryPath == null
            ? "Set LL10_AGENT_FFI to the built agent_ffi.dll to run this test."
            : null;

    test(
        "loads the Rust library and reads real diagnostics",
        () async {
            final client = FfiAgentClient.load(libraryPath: libraryPath);

            expect(client.abiVersion, FfiAgentClient.supportedAbiVersion);

            final diagnostics = await client.readDiagnostics();

            expect(diagnostics.agentVersion, isNotEmpty);
            expect(diagnostics.os, "windows");
            expect(diagnostics.architecture, isNotEmpty);
            expect(diagnostics.windows, isNotNull);
            expect(diagnostics.windows!.build, isNotEmpty);
        },
        skip: skipReason,
    );
}
