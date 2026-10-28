#![cfg_attr(windows, windows_subsystem = "windows")]

use anyhow::Result;
use patch_runtime::{MemoryBackend, PatchRuntime, WindowsBackend};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct Request {
    schema: u32,
    action: String,
    patch_id: String,
    patches_dir: String,
    confirm: bool,
}

#[derive(Debug, Serialize)]
struct ResultEnvelope {
    schema: u32,
    ok: bool,
    code: i32,
    message: String,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut request_path = String::new();
    let mut result_path = String::new();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--request" => {
                if i + 1 < args.len() { request_path = args[i + 1].clone(); i += 1; }
            }
            "--result" => {
                if i + 1 < args.len() { result_path = args[i + 1].clone(); i += 1; }
            }
            _ => {}
        }
        i += 1;
    }

    let exit_code = run(&request_path, &result_path);
    std::process::exit(exit_code);
}

fn run(request_path: &str, result_path: &str) -> i32 {
    // Read request
    let req_str = match std::fs::read_to_string(request_path) {
        Ok(s) => s,
        Err(e) => {
            write_result(result_path, false, 1, &format!("failed to read request: {}", e));
            return 1;
        }
    };
    let req: Request = match serde_json::from_str(&req_str) {
        Ok(r) => r,
        Err(e) => {
            write_result(result_path, false, 1, &format!("invalid request: {}", e));
            return 1;
        }
    };

    if req.schema != 1 {
        write_result(result_path, false, 1, "unsupported schema");
        return 1;
    }

    let dir = PathBuf::from(&req.patches_dir);

    #[cfg(windows)]
    let backend = WindowsBackend::new();
    #[cfg(not(windows))]
    let backend = MemoryBackend::new();

    let runtime = PatchRuntime::new(backend, dir);

    let (ok, code, message) = match req.action.as_str() {
        "apply" => {
            match runtime.apply(&req.patch_id, req.confirm) {
                Ok(report) => (true, 0, report.message),
                Err(e) => (false, 1, e.to_string()),
            }
        }
        "revert" => {
            match runtime.revert(&req.patch_id, req.confirm) {
                Ok(report) => (true, 0, report.message),
                Err(e) => (false, 1, e.to_string()),
            }
        }
        "verify" => {
            match runtime.verify(&req.patch_id) {
                Ok(state) => (true, 0, format!("{:?}", state)),
                Err(e) => (false, 1, e.to_string()),
            }
        }
        _ => (false, 1, format!("unknown action: {}", req.action)),
    };

    write_result(result_path, ok, code, &message);

    // Clean up request file
    let _ = std::fs::remove_file(request_path);

    code
}

fn write_result(path: &str, ok: bool, code: i32, message: &str) {
    let envelope = ResultEnvelope {
        schema: 1,
        ok,
        code,
        message: message.to_string(),
    };
    if let Ok(json) = serde_json::to_string_pretty(&envelope) {
        let tmp = format!("{}.tmp", path);
        let _ = std::fs::write(&tmp, &json);
        let _ = std::fs::rename(&tmp, path);
    }
}
