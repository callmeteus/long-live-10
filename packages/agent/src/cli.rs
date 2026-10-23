//! Command line entry point for diagnostics, planning, apply, and revert.

use crate::diagnostics;
use crate::executor::{self, Journal};
use crate::request::AgentRequest;
use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use patch_core::{PatchEngine, PatchPlan};
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
    }
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
