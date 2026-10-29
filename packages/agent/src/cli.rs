//! Command line entry point for diagnostics, planning, apply, and revert.

use crate::diagnostics;
use crate::executor::{self, Journal};
use crate::request::AgentRequest;
use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use patch_core::{PatchEngine, PatchManifest, PatchPlan};
use std::path::{Path, PathBuf};

/// Long Live 10 command line interface.
#[derive(Debug, Parser)]
#[command(
    name = "ll10-agent",
    version,
    about = "Long Live 10 diagnostics and bounded patch operations"
)]
struct Cli {
    /// Command to run. Diagnostics are the default.
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Reports system information without elevation.
    Diagnostics {
        /// Print the versioned JSON payload instead of a summary.
        #[arg(long)]
        json: bool,
    },
    /// Plans a patch request without changing the system.
    Plan {
        /// Request JSON containing the patch, artifacts, state, and requirements.
        #[arg(long)]
        request: PathBuf,
    },
    /// Applies a planned file replacement. Requires --confirm.
    Apply {
        /// Request JSON containing the patch, artifacts, state, and requirements.
        #[arg(long)]
        request: PathBuf,
        /// Directory containing the artifact files.
        #[arg(long)]
        artifact_dir: PathBuf,
        /// Directory that receives replaced files.
        #[arg(long)]
        target_dir: PathBuf,
        /// Directory that receives the recovery journal and backups.
        #[arg(long)]
        journal: PathBuf,
        /// Actually write files. Without this flag the command is a dry run.
        #[arg(long)]
        confirm: bool,
    },
    /// Restores files from a recovery journal. Requires --confirm.
    Revert {
        /// Journal directory written by apply.
        #[arg(long)]
        journal: PathBuf,
        /// Actually restore files. Without this flag the command only reports.
        #[arg(long)]
        confirm: bool,
    },
    /// Manages declarative patches.
    Patches {
        #[command(subcommand)]
        command: PatchesCommand,
    },
    /// Internal: trusted operation (run as SYSTEM via scheduled task).
    #[command(hide = true)]
    TrustedOp {
        /// Operation: replace or backup.
        op: String,
        /// Source path.
        source: String,
        /// Destination path.
        dest: String,
    },
}

#[derive(Debug, Subcommand)]
enum PatchesCommand {
    /// Lists all patches in the patches directory.
    List {
        /// Directory containing patches.
        #[arg(long)]
        patches_dir: Option<PathBuf>,
        /// Print JSON.
        #[arg(long)]
        json: bool,
    },
    /// Shows a single patch manifest.
    Show {
        /// Patch id.
        id: String,
        /// Directory containing patches.
        #[arg(long)]
        patches_dir: Option<PathBuf>,
        /// Print JSON.
        #[arg(long)]
        json: bool,
    },
    /// Validates all manifests.
    Validate {
        /// Directory containing patches.
        #[arg(long)]
        patches_dir: Option<PathBuf>,
        /// Print JSON.
        #[arg(long)]
        json: bool,
    },
    /// Verifies a patch.
    Verify {
        /// Patch id.
        id: String,
        /// Directory containing patches.
        #[arg(long)]
        patches_dir: Option<PathBuf>,
        /// Print JSON.
        #[arg(long)]
        json: bool,
    },
    /// Applies a patch.
    Apply {
        /// Patch id.
        id: String,
        /// Directory containing patches.
        #[arg(long)]
        patches_dir: Option<PathBuf>,
        /// Actually apply. Without this flag the command is a dry run.
        #[arg(long)]
        confirm: bool,
        /// Print JSON.
        #[arg(long)]
        json: bool,
    },
    /// Re-applies a patch only when the target drifted (for auto-remediation).
    Remediate {
        /// Patch id.
        id: String,
        /// Directory containing patches.
        #[arg(long)]
        patches_dir: Option<PathBuf>,
        /// Print JSON.
        #[arg(long)]
        json: bool,
    },
    /// Reverts a patch.
    Revert {
        /// Patch id.
        id: String,
        /// Directory containing patches.
        #[arg(long)]
        patches_dir: Option<PathBuf>,
        /// Actually revert. Without this flag the command is a dry run.
        #[arg(long)]
        confirm: bool,
        /// Print JSON.
        #[arg(long)]
        json: bool,
    },
}

/// Runs the command line and returns the process result.
pub fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Command::Diagnostics { json: false }) {
        Command::Diagnostics { json } => diagnostics_command(json),
        Command::Plan { request } => plan_command(&request),
        Command::Apply {
            request,
            artifact_dir,
            target_dir,
            journal,
            confirm,
        } => apply_command(&request, &artifact_dir, &target_dir, &journal, confirm),
        Command::Revert { journal, confirm } => revert_command(&journal, confirm),
        Command::Patches { command } => patches_command(command),
        Command::TrustedOp { op, source, dest } => trusted_op_command(&op, &source, &dest),
    }
}

fn patches_command(command: PatchesCommand) -> Result<()> {
    match command {
        PatchesCommand::List { patches_dir, json } => {
            let dir = patches_dir.unwrap_or_else(default_patches_dir);
            let mut ids = Vec::new();
            for entry in std::fs::read_dir(&dir).context("read patches dir")? {
                let entry = entry?;
                if entry.path().is_dir() {
                    if entry.path().join("patch.yml").is_file() {
                        ids.push(entry.file_name().to_string_lossy().to_string());
                    }
                }
            }
            ids.sort();
            if json {
                println!("{}", serde_json::to_string_pretty(&ids)?);
            } else {
                for id in &ids { println!("{}", id); }
            }
            Ok(())
        }
        PatchesCommand::Show { id, patches_dir, json } => {
            let dir = patches_dir.unwrap_or_else(default_patches_dir);
            let path = dir.join(&id).join("patch.yml");
            let manifest = PatchManifest::load_from_path(&path)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&manifest)?);
            } else {
                println!("{} - {}", manifest.id, manifest.title);
                println!("severity: {}", manifest.severity);
                println!("target: {:?}", manifest.target);
                println!("operations: {}", manifest.operations.len());
                println!("checks: {}", manifest.checks.len());
            }
            Ok(())
        }
        PatchesCommand::Validate { patches_dir, json } => {
            let dir = patches_dir.unwrap_or_else(default_patches_dir);
            let mut results = Vec::new();
            for entry in std::fs::read_dir(&dir).context("read patches dir")? {
                let entry = entry?;
                if !entry.path().is_dir() { continue; }
                let manifest_path = entry.path().join("patch.yml");
                if !manifest_path.is_file() { continue; }
                let id = entry.file_name().to_string_lossy().to_string();
                match PatchManifest::load_from_path(&manifest_path) {
                    Ok(m) => {
                        // check id matches folder
                        if m.id != id {
                            results.push((id.clone(), false, format!("id mismatch: {} != {}", m.id, id)));
                        } else {
                            results.push((id.clone(), true, "ok".to_string()));
                        }
                    }
                    Err(e) => results.push((id.clone(), false, e.to_string())),
                }
            }
            results.sort_by(|a, b| a.0.cmp(&b.0));
            let all_ok = results.iter().all(|r| r.1);
            if json {
                let obj: Vec<serde_json::Value> = results.iter().map(|r| serde_json::json!({"id": r.0, "ok": r.1, "message": r.2})).collect();
                println!("{}", serde_json::to_string_pretty(&obj)?);
            } else {
                for r in &results {
                    println!("{}: {} - {}", if r.1 { "PASS" } else { "FAIL" }, r.0, r.2);
                }
            }
            if !all_ok { anyhow::bail!("validation failed"); }
            Ok(())
        }
        PatchesCommand::Verify { id, patches_dir, json } => {
            let dir = patches_dir.unwrap_or_else(default_patches_dir);
            #[cfg(windows)]
            let backend = patch_runtime::WindowsBackend::new();
            #[cfg(not(windows))]
            let backend = patch_runtime::MemoryBackend::new();
            let runtime = patch_runtime::PatchRuntime::new(backend, dir);
            let state = runtime.verify(&id)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&serde_json::json!({"patch_id": id, "state": format!("{:?}", state)}))?);
            } else {
                println!("{}: {:?}", id, state);
            }
            Ok(())
        }
        PatchesCommand::Apply { id, patches_dir, confirm, json } => {
            let dir = patches_dir.unwrap_or_else(default_patches_dir);
            #[cfg(windows)]
            let backend = patch_runtime::WindowsBackend::new();
            #[cfg(not(windows))]
            let backend = patch_runtime::MemoryBackend::new();
            let runtime = patch_runtime::PatchRuntime::new(backend, dir);
            let report = runtime.apply(&id, confirm)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("{}", report.message);
            }
            Ok(())
        }
        PatchesCommand::Remediate { id, patches_dir, json } => {
            let dir = patches_dir.unwrap_or_else(default_patches_dir);
            #[cfg(windows)]
            let backend = patch_runtime::WindowsBackend::new();
            #[cfg(not(windows))]
            let backend = patch_runtime::MemoryBackend::new();
            let runtime = patch_runtime::PatchRuntime::new(backend, dir);

            let before = runtime.verify(&id)?;
            // The subscription only re-applies patches whose journal says they
            // are installed: a revert must not be undone by a late event.
            let (after, action) = match runtime.remediate(&id)? {
                Some(report) if report.state != before => (report.state, "applied"),
                _ => (before.clone(), "none"),
            };

            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "patch_id": id,
                        "state_before": format!("{:?}", before),
                        "state_after": format!("{:?}", after),
                        "action": action,
                    }))?
                );
            } else {
                println!("{}: {} -> {} ({})", id, format!("{:?}", before), format!("{:?}", after), action);
            }
            Ok(())
        }
        PatchesCommand::Revert { id, patches_dir, confirm, json } => {
            let dir = patches_dir.unwrap_or_else(default_patches_dir);
            #[cfg(windows)]
            let backend = patch_runtime::WindowsBackend::new();
            #[cfg(not(windows))]
            let backend = patch_runtime::MemoryBackend::new();
            let runtime = patch_runtime::PatchRuntime::new(backend, dir);
            let report = runtime.revert(&id, confirm)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("{}", report.message);
            }
            Ok(())
        }
    }
}

fn trusted_op_command(op: &str, source: &str, dest: &str) -> Result<()> {
    // When running as SYSTEM (via scheduled task), directly impersonate TrustedInstaller
    match op {
        "replace" => {
            patch_runtime::trusted_installer::run_as_trusted_installer(|| {
                // Use MoveFileEx with DELAY_UNTIL_REBOOT for in-use files
                use std::ffi::OsStr;
                use std::os::windows::ffi::OsStrExt;
                #[link(name = "kernel32")]
                extern "system" {
                    fn MoveFileExW(lpExistingFileName: *const u16, lpNewFileName: *const u16, dwFlags: u32) -> i32;
                }
                const MOVEFILE_REPLACE_EXISTING: u32 = 0x00000001;
                const MOVEFILE_DELAY_UNTIL_REBOOT: u32 = 0x00000004;
                let src: Vec<u16> = OsStr::new(source).encode_wide().chain(std::iter::once(0)).collect();
                let dst: Vec<u16> = OsStr::new(dest).encode_wide().chain(std::iter::once(0)).collect();
                let result = unsafe {
                    MoveFileExW(src.as_ptr(), dst.as_ptr(), MOVEFILE_REPLACE_EXISTING | MOVEFILE_DELAY_UNTIL_REBOOT)
                };
                if result == 0 {
                    return Err(format!("MoveFileEx failed: {}", std::io::Error::last_os_error()));
                }
                Ok(())
            }).map_err(|e| anyhow::anyhow!("{}", e))?;
        }
        "backup" => {
            patch_runtime::trusted_installer::run_as_trusted_installer(|| {
                std::fs::copy(source, dest).map_err(|e| format!("backup failed: {}", e))?;
                Ok(())
            }).map_err(|e| anyhow::anyhow!("{}", e))?;
        }
        _ => anyhow::bail!("unknown op: {}", op),
    }
    Ok(())
}

fn default_patches_dir() -> PathBuf {
    // Walk up from current dir to find packages/patches
    let mut cur = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    for _ in 0..6 {
        let candidate = cur.join("packages").join("patches");
        if candidate.is_dir() { return candidate; }
        if let Some(parent) = cur.parent() { cur = parent.to_path_buf(); } else { break; }
    }
    PathBuf::from("packages/patches")
}

fn diagnostics_command(json: bool) -> Result<()> {
    let info = diagnostics();

    if json {
        println!("{}", info.to_json().context("diagnostics encode")?);

        return Ok(());
    }

    println!("Long Live 10 agent {}", info.agent_version);
    println!("OS: {} ({})", info.os, info.architecture);

    if let Some(windows) = info.windows {
        let revision = windows
            .update_build_revision
            .map(|value| format!(".{value}"))
            .unwrap_or_default();

        println!(
            "Windows: {} {} ({}{})",
            windows.product_name, windows.display_version, windows.build, revision
        );
    } else {
        println!("Windows: not detected");
    }

    Ok(())
}

fn plan_command(request_path: &Path) -> Result<()> {
    let request = AgentRequest::from_path(request_path)?;
    let plan = PatchEngine::new().plan(&request.plan_request());

    print_plan(&plan)?;

    ensure_plan_is_safe(&plan)
}

fn apply_command(
    request_path: &Path,
    artifact_dir: &Path,
    target_dir: &Path,
    journal_dir: &Path,
    confirm: bool,
) -> Result<()> {
    let request = AgentRequest::from_path(request_path)?;
    let plan = PatchEngine::new().plan(&request.plan_request());

    print_plan(&plan)?;
    ensure_plan_is_safe(&plan)?;

    if !confirm {
        eprintln!("dry run: pass --confirm to apply this plan");

        return Ok(());
    }

    let journal = executor::apply(&request, &plan, artifact_dir, target_dir, journal_dir)
        .context("failed to apply plan")?;

    println!("journal: {}", journal_dir.display());
    println!("changes: {}", journal.changes.len());

    Ok(())
}

fn revert_command(journal_dir: &Path, confirm: bool) -> Result<()> {
    let journal = Journal::read(journal_dir).context("failed to read journal")?;

    println!("patch: {}", journal.patch_id);
    println!("changes: {}", journal.changes.len());

    if journal.reverted_at.is_some() {
        bail!("journal has already been reverted");
    }

    if !confirm {
        eprintln!("dry run: pass --confirm to restore the journal changes");

        return Ok(());
    }

    let restored = executor::revert(journal_dir).context("failed to revert journal")?;

    println!("reverted: {restored}");

    Ok(())
}

fn ensure_plan_is_safe(plan: &PatchPlan) -> Result<()> {
    if !plan.conflicts.is_empty() {
        bail!("plan has {} conflict(s)", plan.conflicts.len());
    }

    if !plan.recovery_ready {
        bail!("plan is not recovery ready");
    }

    Ok(())
}

fn print_plan(plan: &PatchPlan) -> Result<()> {
    let json = serde_json::to_string_pretty(plan).context("plan encode")?;

    println!("{json}");

    Ok(())
}
