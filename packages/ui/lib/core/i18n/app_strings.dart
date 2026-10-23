/// Localized strings for the Long Live 10 desktop interface.
class AppStrings {
    const AppStrings({required this.common, required this.diagnostics});

    /// Strings shared by more than one screen.
    final CommonStrings common;

    /// Strings owned by the diagnostics screen.
    final DiagnosticsStrings diagnostics;

    /// English (en-US) strings.
    static const AppStrings en = AppStrings(
        common: CommonStrings(appTitle: "Long Live 10"),
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
    );

    /// Brazilian Portuguese (pt-BR) strings.
    static const AppStrings ptBr = AppStrings(
        common: CommonStrings(appTitle: "Long Live 10"),
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
    );
}

/// Strings shared across screens.
class CommonStrings {
    const CommonStrings({required this.appTitle});

    final String appTitle;
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
