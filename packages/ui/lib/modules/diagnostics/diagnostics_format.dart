/// User-facing label for the operating system identifier reported by the agent.
String formatOperatingSystem(String identifier) {
    switch (identifier.toLowerCase()) {
        case "windows":
            return "Windows";
        case "linux":
            return "Linux";
        case "macos":
        case "darwin":
            return "macOS";
        default:
            if (identifier.isEmpty) {
                return identifier;
            }

            return identifier[0].toUpperCase() + identifier.substring(1);
    }
}
