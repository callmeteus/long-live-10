#[cfg(windows)]
fn is_system() -> bool {
    use std::process::Command;
    let output = Command::new("whoami").output();
    if let Ok(o) = output {
        let user = String::from_utf8_lossy(&o.stdout);
        return user.trim().eq_ignore_ascii_case("nt authority\\system");
    }
    false
}

#[cfg(windows)]
fn delegate_to_system_task(op: &str, arg1: &str, arg2: &str) -> Result<(), String> {
    let task_name = "LL10_System_Op";
    let agent_path = std::env::current_exe().unwrap_or_default();
    let cmd = format!(
        "\"{}\" trusted-op {} \"{}\" \"{}\"",
        agent_path.display(), op, arg1, arg2
    );

    let _ = std::process::Command::new("schtasks")
        .args(["/delete", "/tn", task_name, "/f"])
        .output();

    let create = std::process::Command::new("schtasks")
        .args([
            "/create", "/tn", task_name,
            "/tr", &cmd,
            "/sc", "once",
            "/st", "23:59",
            "/ru", "SYSTEM",
            "/rl", "HIGHEST",
            "/f",
        ])
        .output()
        .map_err(|e| format!("create task failed: {}", e))?;

    if !create.status.success() {
        return Err(format!("create task failed: {}", String::from_utf8_lossy(&create.stderr)));
    }

    let run = std::process::Command::new("schtasks")
        .args(["/run", "/tn", task_name])
        .output()
        .map_err(|e| format!("run task failed: {}", e))?;

    if !run.status.success() {
        let _ = std::process::Command::new("schtasks")
            .args(["/delete", "/tn", task_name, "/f"])
            .output();
        return Err(format!("run task failed: {}", String::from_utf8_lossy(&run.stderr)));
    }

    for _ in 0..60 {
        std::thread::sleep(std::time::Duration::from_millis(500));
        let query = std::process::Command::new("schtasks")
            .args(["/query", "/tn", task_name, "/fo", "CSV", "/nh"])
            .output();
        if let Ok(q) = query {
            let output = String::from_utf8_lossy(&q.stdout);
            if output.contains("Ready") {
                break;
            }
        }
    }

    let _ = std::process::Command::new("schtasks")
        .args(["/delete", "/tn", task_name, "/f"])
        .output();

    if std::path::Path::new(arg2).exists() {
        Ok(())
    } else {
        Err("operation was not completed".to_string())
    }
}

#[cfg(windows)]
pub fn replace_file_trusted(source: &str, dest: &str) -> Result<(), String> {
    if is_system() {
        // Running as SYSTEM, impersonate TrustedInstaller
        run_as_trusted_installer(|| {
            std::fs::copy(source, dest).map_err(|e| format!("copy failed: {}", e))?;
            Ok(())
        })
    } else {
        // Not SYSTEM, delegate to a scheduled task running as SYSTEM
        delegate_to_system_task("replace", source, dest)
    }
}

#[cfg(windows)]
pub fn backup_file_trusted(path: &str, backup: &str) -> Result<(), String> {
    if is_system() {
        run_as_trusted_installer(|| {
            std::fs::copy(path, backup).map_err(|e| format!("backup failed: {}", e))?;
            Ok(())
        })
    } else {
        delegate_to_system_task("backup", path, backup)
    }
}
#[cfg(windows)]
fn enable_debug_privilege() {
    use std::mem;
    #[link(name = "advapi32")]
    extern "system" {
        fn OpenProcessToken(ProcessHandle: *mut std::ffi::c_void, DesiredAccess: u32, TokenHandle: *mut *mut std::ffi::c_void) -> i32;
        fn LookupPrivilegeValueW(lpSystemName: *const u16, lpName: *const u16, lpLuid: *mut Luid) -> i32;
        fn AdjustTokenPrivileges(TokenHandle: *mut std::ffi::c_void, DisableAllPrivileges: i32, NewState: *const TokenPrivileges, BufferLength: u32, PreviousState: *mut std::ffi::c_void, ReturnLength: *mut u32) -> i32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcess() -> *mut std::ffi::c_void;
    }
    #[repr(C)]
    struct Luid { low_part: u32, high_part: i32 }
    #[repr(C)]
    struct TokenPrivileges { privilege_count: u32, privilege: Luid, attributes: u32 }
    const TOKEN_ADJUST_PRIVILEGES: u32 = 0x0020;
    const TOKEN_QUERY: u32 = 0x0008;
    const SE_PRIVILEGE_ENABLED: u32 = 0x00000002;
    unsafe {
        let mut token: *mut std::ffi::c_void = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &mut token) == 0 { return; }
        let mut luid = Luid { low_part: 0, high_part: 0 };
        let name: Vec<u16> = "SeDebugPrivilege".encode_utf16().chain(std::iter::once(0)).collect();
        if LookupPrivilegeValueW(std::ptr::null(), name.as_ptr(), &mut luid) == 0 { return; }
        let tp = TokenPrivileges { privilege_count: 1, privilege: luid, attributes: SE_PRIVILEGE_ENABLED };
        AdjustTokenPrivileges(token, 0, &tp, 0, std::ptr::null_mut(), std::ptr::null_mut());
    }
}

#[cfg(windows)]
pub fn run_as_trusted_installer<F, R>(f: F) -> Result<R, String>
where F: FnOnce() -> Result<R, String>,
{
    enable_debug_privilege();
    // Start TrustedInstaller service
    let _ = std::process::Command::new("sc").args(["start", "TrustedInstaller"]).output();
    std::thread::sleep(std::time::Duration::from_secs(2));
    // Find PID
    let output = std::process::Command::new("tasklist").args(["/FI", "IMAGENAME eq TrustedInstaller.exe", "/FO", "CSV", "/NH"]).output().map_err(|e| format!("{}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut pid: u32 = 0;
    for line in stdout.lines() {
        if line.contains("TrustedInstaller") {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 2 {
                if let Ok(p) = parts[1].trim().trim_matches('"').parse::<u32>() { pid = p; break; }
            }
        }
    }
    if pid == 0 { return Err("TrustedInstaller process not found".to_string()); }
    // Open process and token
    #[link(name = "kernel32")]
    extern "system" { fn OpenProcess(da: u32, ih: i32, pid: u32) -> *mut std::ffi::c_void; fn CloseHandle(h: *mut std::ffi::c_void) -> i32; }
    #[link(name = "advapi32")]
    extern "system" {
        fn OpenProcessToken(ph: *mut std::ffi::c_void, da: u32, th: *mut *mut std::ffi::c_void) -> i32;
        fn DuplicateTokenEx(et: *mut std::ffi::c_void, da: u32, ta: *const std::ffi::c_void, il: i32, tt: u32, nt: *mut *mut std::ffi::c_void) -> i32;
        fn SetThreadToken(t: *mut std::ffi::c_void, tok: *mut std::ffi::c_void) -> i32;
        fn RevertToSelf() -> i32;
    }
    const PROCESS_QUERY_INFORMATION: u32 = 0x0400;
    const TOKEN_DUPLICATE: u32 = 0x0002;
    const TOKEN_IMPERSONATE: u32 = 0x0004;
    const TOKEN_QUERY: u32 = 0x0008;
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_INFORMATION, 0, pid);
        if process.is_null() { return Err(format!("OpenProcess failed: {}", std::io::Error::last_os_error())); }
        let mut token: *mut std::ffi::c_void = std::ptr::null_mut();
        if OpenProcessToken(process, TOKEN_DUPLICATE | TOKEN_IMPERSONATE | TOKEN_QUERY, &mut token) == 0 {
            CloseHandle(process);
            return Err(format!("OpenProcessToken failed: {}", std::io::Error::last_os_error()));
        }
        let mut dup: *mut std::ffi::c_void = std::ptr::null_mut();
        if DuplicateTokenEx(token, 0x000F01FF, std::ptr::null(), 2, 2, &mut dup) == 0 {
            CloseHandle(token); CloseHandle(process);
            return Err(format!("DuplicateTokenEx failed: {}", std::io::Error::last_os_error()));
        }
        if SetThreadToken(std::ptr::null_mut(), dup) == 0 {
            CloseHandle(dup); CloseHandle(token); CloseHandle(process);
            return Err(format!("SetThreadToken failed: {}", std::io::Error::last_os_error()));
        }
        let result = f();
        RevertToSelf();
        CloseHandle(dup); CloseHandle(token); CloseHandle(process);
        result
    }
}
