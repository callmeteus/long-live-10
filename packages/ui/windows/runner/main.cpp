#include <flutter/dart_project.h>
#include <flutter/flutter_view_controller.h>
#include <flutter_windows.h>
#include <windows.h>

#include "flutter_window.h"
#include "utils.h"

namespace {

// Centers the window in the primary monitor work area, excluding the taskbar.
Win32Window::Point GetCenteredOrigin(const Win32Window::Size& size) {
    RECT work_area{};
    if (!::SystemParametersInfo(SPI_GETWORKAREA, 0, &work_area, 0)) {
        return Win32Window::Point(10, 10);
    }

    const POINT center = {
            static_cast<LONG>((work_area.left + work_area.right) / 2),
            static_cast<LONG>((work_area.top + work_area.bottom) / 2)};
    HMONITOR monitor = ::MonitorFromPoint(center, MONITOR_DEFAULTTOPRIMARY);
    const double scale_factor = FlutterDesktopGetDpiForMonitor(monitor) / 96.0;

    const int window_width = static_cast<int>(size.width * scale_factor);
    const int window_height = static_cast<int>(size.height * scale_factor);
    const int x =
            work_area.left + ((work_area.right - work_area.left) - window_width) / 2;
    const int y =
            work_area.top + ((work_area.bottom - work_area.top) - window_height) / 2;

    return Win32Window::Point(static_cast<int>(x / scale_factor),
                                                        static_cast<int>(y / scale_factor));
}

}  // namespace

int APIENTRY wWinMain(_In_ HINSTANCE instance, _In_opt_ HINSTANCE prev,
                                            _In_ wchar_t *command_line, _In_ int show_command) {
    // Attach to console when present (e.g., 'flutter run') or create a
    // new console when running with a debugger.
    if (!::AttachConsole(ATTACH_PARENT_PROCESS) && ::IsDebuggerPresent()) {
        CreateAndAttachConsole();
    }

    // Initialize COM, so that it is available for use in the library and/or
    // plugins.
    ::CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);

    flutter::DartProject project(L"data");

    std::vector<std::string> command_line_arguments =
            GetCommandLineArguments();

    project.set_dart_entrypoint_arguments(std::move(command_line_arguments));

    FlutterWindow window(project);
    Win32Window::Size size(1280, 720);
    Win32Window::Point origin = GetCenteredOrigin(size);
    if (!window.Create(L"Long Live 10", origin, size)) {
        return EXIT_FAILURE;
    }
    window.SetQuitOnClose(true);

    ::MSG msg;
    while (::GetMessage(&msg, nullptr, 0, 0)) {
        ::TranslateMessage(&msg);
        ::DispatchMessage(&msg);
    }

    ::CoUninitialize();
    return EXIT_SUCCESS;
}
