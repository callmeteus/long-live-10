/// Localized strings for the Long Live 10 desktop interface.
class AppStrings {
    const AppStrings({
        required this.common,
        required this.diagnostics,
        required this.status,
    });

    /// Strings shared by more than one screen.
    final CommonStrings common;

    /// Strings owned by the diagnostics screen.
    final DiagnosticsStrings diagnostics;

    /// Strings owned by the status screen.
    final StatusStrings status;

    /// English (en-US) strings.
    static const AppStrings en = AppStrings(
        common: CommonStrings(
            appTitle: "Long Live 10",
            statusTab: "Status",
            diagnosticsTab: "Diagnostics",
        ),
        diagnostics: DiagnosticsStrings(
            title: "System diagnostics",
            subtitle: "Device information and the Windows version.",
            refresh: "Refresh",
            loading: "Reading diagnostics",
            errorTitle: "Could not read diagnostics",
            errorHint: "Check that the agent library is available and try again.",
            retry: "Try again",
            agentVersion: "Agent version",
            operatingSystem: "Operating system",
            architecture: "Architecture",
            windowsVersion: "Windows version",
            build: "Windows build",
            notReported: "Not reported",
        ),
        status: StatusStrings(
            title: "System status",
            subtitle: "Whether this PC can receive supported fixes.",
            refresh: "Refresh",
            loading: "Checking this PC",
            errorTitle: "Could not check this PC",
            errorHint: "The agent did not report enough information. Try again.",
            retry: "Try again",
            supportedTitle: "This PC is supported",
            supportedBody: "Windows 10 22H2 on 64-bit is covered by this release.",
            unsupportedTitle: "This PC is not supported",
            unsupportedBody:
                    "This release covers Windows 10 22H2 on 64-bit. Nothing will be changed on this PC.",
            unknownTitle: "Status unavailable",
            unknownBody: "The Windows version could not be read. Refresh to try again.",
            windowsVersion: "Windows version",
            currentBuild: "Current build",
            architecture: "Architecture",
            agentVersion: "Agent version",
            notReported: "Not reported",
        ),
    );

    /// Brazilian Portuguese (pt-BR) strings.
    static const AppStrings ptBr = AppStrings(
        common: CommonStrings(
            appTitle: "Long Live 10",
            statusTab: "Status",
            diagnosticsTab: "Diagnóstico",
        ),
        diagnostics: DiagnosticsStrings(
            title: "Diagnóstico do sistema",
            subtitle: "Informações do dispositivo e da versão do Windows.",
            refresh: "Atualizar",
            loading: "Lendo diagnóstico",
            errorTitle: "Não foi possível ler o diagnóstico",
            errorHint: "Verifique se a biblioteca do agente está disponível e tente novamente.",
            retry: "Tentar novamente",
            agentVersion: "Versão do agente",
            operatingSystem: "Sistema operacional",
            architecture: "Arquitetura",
            windowsVersion: "Versão do Windows",
            build: "Compilação do Windows",
            notReported: "Não informado",
        ),
        status: StatusStrings(
            title: "Status do sistema",
            subtitle: "Verifica se este PC pode receber correções suportadas.",
            refresh: "Atualizar",
            loading: "Verificando este PC",
            errorTitle: "Não foi possível verificar este PC",
            errorHint: "O agente não informou dados suficientes. Tente novamente.",
            retry: "Tentar novamente",
            supportedTitle: "Este PC é compatível",
            supportedBody: "O Windows 10 22H2 de 64 bits é coberto por esta versão.",
            unsupportedTitle: "Este PC não é compatível",
            unsupportedBody:
                    "Esta versão cobre o Windows 10 22H2 de 64 bits. Nada será alterado neste PC.",
            unknownTitle: "Status indisponível",
            unknownBody: "Não foi possível ler a versão do Windows. Atualize para tentar novamente.",
            windowsVersion: "Versão do Windows",
            currentBuild: "Compilação atual",
            architecture: "Arquitetura",
            agentVersion: "Versão do agente",
            notReported: "Não informado",
        ),
    );
}

/// Strings shared across screens.
class CommonStrings {
    const CommonStrings({
        required this.appTitle,
        required this.statusTab,
        required this.diagnosticsTab,
    });

    final String appTitle;
    final String statusTab;
    final String diagnosticsTab;
}

/// Strings used by the diagnostics screen.
class DiagnosticsStrings {
    const DiagnosticsStrings({
        required this.title,
        required this.subtitle,
        required this.refresh,
        required this.loading,
        required this.errorTitle,
        required this.errorHint,
        required this.retry,
        required this.agentVersion,
        required this.operatingSystem,
        required this.architecture,
        required this.windowsVersion,
        required this.build,
        required this.notReported,
    });

    final String title;
    final String subtitle;
    final String refresh;
    final String loading;
    final String errorTitle;
    final String errorHint;
    final String retry;
    final String agentVersion;
    final String operatingSystem;
    final String architecture;
    final String windowsVersion;
    final String build;
    final String notReported;
}

/// Strings used by the status screen.
class StatusStrings {
    const StatusStrings({
        required this.title,
        required this.subtitle,
        required this.refresh,
        required this.loading,
        required this.errorTitle,
        required this.errorHint,
        required this.retry,
        required this.supportedTitle,
        required this.supportedBody,
        required this.unsupportedTitle,
        required this.unsupportedBody,
        required this.unknownTitle,
        required this.unknownBody,
        required this.windowsVersion,
        required this.currentBuild,
        required this.architecture,
        required this.agentVersion,
        required this.notReported,
    });

    final String title;
    final String subtitle;
    final String refresh;
    final String loading;
    final String errorTitle;
    final String errorHint;
    final String retry;
    final String supportedTitle;
    final String supportedBody;
    final String unsupportedTitle;
    final String unsupportedBody;
    final String unknownTitle;
    final String unknownBody;
    final String windowsVersion;
    final String currentBuild;
    final String architecture;
    final String agentVersion;
    final String notReported;
}
