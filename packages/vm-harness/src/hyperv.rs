use anyhow::{bail, Context, Result};
use std::process::Command;

/// Captured result of one host command execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    /// Whether the process exited with a zero status.
    pub success: bool,
    /// Process exit code when the host reported one.
    pub exit_code: Option<i32>,
    /// Captured standard output.
    pub stdout: String,
    /// Captured standard error.
    pub stderr: String,
}

impl CommandOutput {
    /// Turns a failed command into an error that includes the captured output.
    fn into_success(self, context: &str) -> Result<Self> {
        if self.success {
            return Ok(self);
        }

        let detail = if self.stderr.trim().is_empty() {
            self.stdout.trim()
        } else {
            self.stderr.trim()
        };

        bail!("{context}: {detail} (exit code {:?})", self.exit_code);
    }
}

/// Describes a host command to execute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandRequest {
    /// Executable name or path.
    pub program: String,
    /// Arguments passed to the executable.
    pub args: Vec<String>,
}

impl CommandRequest {
    /// Builds a request from a program and an iterator of arguments.
    pub fn new<I, S>(program: impl Into<String>, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            program: program.into(),
            args: args.into_iter().map(Into::into).collect(),
        }
    }
}

/// Executes host programs for the Hyper-V provider.
///
/// Production code uses [`SystemCommandRunner`]. Tests provide a recording
/// implementation so the generated PowerShell can be asserted without a host
/// Hyper-V installation.
pub trait CommandRunner: Send + Sync {
    /// Runs `request` and returns its captured output.
    fn run(&self, request: &CommandRequest) -> Result<CommandOutput>;
}

/// Executes commands with `std::process::Command`.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemCommandRunner;

impl CommandRunner for SystemCommandRunner {
    fn run(&self, request: &CommandRequest) -> Result<CommandOutput> {
        let output = Command::new(&request.program)
            .args(&request.args)
            .output()
            .with_context(|| format!("failed to start {}", request.program))?;

        Ok(CommandOutput {
            success: output.status.success(),
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

/// Quotes a value for a PowerShell single-quoted string.
fn quote_powershell(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

/// Builds a PowerShell request that stops on the first error.
fn powershell(script: impl Into<String>) -> CommandRequest {
    let script: String = script.into();
    let mut request = CommandRequest::new(
        "powershell",
        [
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
        ],
    );

    request
        .args
        .push(format!("$ErrorActionPreference = 'Stop'; {script}"));

    request
}

/// Parameters used to create a new Hyper-V virtual machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HyperVCreateOptions {
    /// Startup memory in megabytes.
    pub memory_mb: u64,
    /// Number of virtual processors.
    pub cpu_count: u32,
    /// Path to the base virtual hard disk.
    pub vhd_path: String,
    /// Optional virtual switch name.
    pub switch_name: Option<String>,
    /// Hyper-V generation, usually 2 for Windows 10 22H2.
    pub generation: u32,
}

impl HyperVCreateOptions {
    /// Builds options with a second generation VM and no switch.
    pub fn new(memory_mb: u64, cpu_count: u32, vhd_path: impl Into<String>) -> Self {
        Self {
            memory_mb,
            cpu_count,
            vhd_path: vhd_path.into(),
            switch_name: None,
            generation: 2,
        }
    }
}

/// Hyper-V operations exposed through PowerShell cmdlets.
///
/// The provider only builds and executes host commands. It never touches the
/// host operating system by itself, so a disposable VM is required for any
/// destructive lifecycle call.
#[derive(Debug, Clone)]
pub struct HyperVProvider<R> {
    runner: R,
    vm_name: String,
}

impl<R: CommandRunner> HyperVProvider<R> {
    /// Creates a provider for the VM named `vm_name`.
    pub fn new(runner: R, vm_name: impl Into<String>) -> Self {
        Self {
            runner,
            vm_name: vm_name.into(),
        }
    }

    /// Returns the VM name handled by this provider.
    pub fn vm_name(&self) -> &str {
        &self.vm_name
    }

    /// Checks whether the Hyper-V PowerShell module is available.
    pub fn detect(&self) -> Result<bool> {
        let output = self
            .runner
            .run(&powershell("Get-Command Get-VM | Out-Null"))?;

        Ok(output.success)
    }

    /// Checks whether the configured VM exists.
    ///
    /// Any lookup failure is treated as a missing VM. A permission failure
    /// surfaces again with a clear message when creation is attempted.
    pub fn exists(&self) -> Result<bool> {
        let script = format!(
            "Get-VM -Name {} | Out-Null",
            quote_powershell(&self.vm_name)
        );
        let output = self.runner.run(&powershell(script))?;

        Ok(output.success)
    }

    /// Creates a VM from a prepared base disk and starts it.
    pub fn create(&self, options: &HyperVCreateOptions) -> Result<()> {
        if options.memory_mb == 0 {
            bail!("VM memory must be greater than zero");
        }

        if options.cpu_count == 0 {
            bail!("VM processor count must be greater than zero");
        }

        if options.vhd_path.trim().is_empty() {
            bail!("VM base disk path must not be empty");
        }

        let memory_bytes = options
            .memory_mb
            .checked_mul(1024 * 1024)
            .context("VM memory size is too large")?;

        let switch = options
            .switch_name
            .as_deref()
            .map(|name| format!(" -SwitchName {}", quote_powershell(name)))
            .unwrap_or_default();

        let create_script = format!(
            "New-VM -Name {} -MemoryStartupBytes {} -Generation {} -VHDPath {}{}",
            quote_powershell(&self.vm_name),
            memory_bytes,
            options.generation,
            quote_powershell(&options.vhd_path),
            switch
        );

        self.runner
            .run(&powershell(create_script))?
            .into_success("failed to create VM")?;

        let processor_script = format!(
            "Set-VMProcessor -VMName {} -Count {}",
            quote_powershell(&self.vm_name),
            options.cpu_count
        );

        self.runner
            .run(&powershell(processor_script))?
            .into_success("failed to configure VM processors")?;

        self.start()
    }

    /// Restores the VM to a named checkpoint and starts it.
    pub fn restore_checkpoint(&self, checkpoint: &str) -> Result<()> {
        if checkpoint.trim().is_empty() {
            bail!("checkpoint name must not be empty");
        }

        let script = format!(
            "Restore-VMSnapshot -VMName {} -Name {} -Confirm:$false",
            quote_powershell(&self.vm_name),
            quote_powershell(checkpoint)
        );

        self.runner
            .run(&powershell(script))?
            .into_success("failed to restore VM checkpoint")?;

        self.start()
    }

    /// Starts the VM when it is not already running.
    pub fn start(&self) -> Result<()> {
        let script = format!("Start-VM -Name {}", quote_powershell(&self.vm_name));

        self.runner
            .run(&powershell(script))?
            .into_success("failed to start VM")?;

        Ok(())
    }

    /// Stops the VM immediately.
    pub fn stop(&self) -> Result<()> {
        let script = format!(
            "Stop-VM -Name {} -TurnOff -Force",
            quote_powershell(&self.vm_name)
        );

        self.runner
            .run(&powershell(script))?
            .into_success("failed to stop VM")?;

        Ok(())
    }

    /// Removes the VM and its configuration.
    ///
    /// The VM is stopped first. Stop failures are ignored because an already
    /// stopped VM is the common case during teardown.
    pub fn teardown(&self) -> Result<()> {
        let quoted_name = quote_powershell(&self.vm_name);
        let script = format!(
            "Stop-VM -Name {quoted_name} -TurnOff -Force -ErrorAction SilentlyContinue; \
             Remove-VM -Name {quoted_name} -Force"
        );

        self.runner
            .run(&powershell(script))?
            .into_success("failed to remove VM")?;

        Ok(())
    }

    /// Executes a command inside the guest through PowerShell Direct.
    ///
    /// The command runs with the credentials available to the host process.
    /// Use [`HyperVProvider::execute_command_with_credential`] when the guest
    /// uses a dedicated local account.
    pub fn execute_command(&self, command: &str) -> Result<String> {
        self.run_guest_command(command, None)
    }

    /// Executes a command with credentials imported from a DPAPI file.
    ///
    /// The file is created on the host with `Get-Credential | Export-Clixml`
    /// and can only be read by the same host user. The password never appears
    /// in the generated command line.
    pub fn execute_command_with_credential(
        &self,
        command: &str,
        credential_path: &str,
    ) -> Result<String> {
        if credential_path.trim().is_empty() {
            bail!("guest credential path must not be empty");
        }

        let credential = format!(
            "$credential = Import-Clixml {}; ",
            quote_powershell(credential_path)
        );

        self.run_guest_command(command, Some(credential))
    }

    /// Runs a guest command with an optional credential preamble.
    fn run_guest_command(&self, command: &str, credential: Option<String>) -> Result<String> {
        if command.trim().is_empty() {
            bail!("guest command must not be empty");
        }

        let credential_arg = if credential.is_some() {
            " -Credential $credential"
        } else {
            ""
        };

        let script = format!(
            "{}Invoke-Command -VMName {}{} -ScriptBlock {{ param($command) \
             Invoke-Expression $command }} -ArgumentList {}",
            credential.unwrap_or_default(),
            quote_powershell(&self.vm_name),
            credential_arg,
            quote_powershell(command)
        );

        let output = self
            .runner
            .run(&powershell(script))?
            .into_success("guest command failed")?;

        Ok(output.stdout.trim_end().to_string())
    }

    /// Captures recent Hyper-V worker events for this VM.
    pub fn capture_logs(&self) -> Result<String> {
        let script = format!(
            "$vmName = {}; \
             Get-WinEvent -FilterHashtable @{{LogName='Microsoft-Windows-Hyper-V-VMMS-Admin'; \
             StartTime=(Get-Date).AddHours(-2)}} -ErrorAction SilentlyContinue | \
             Where-Object {{ $_.Message -like \"*$vmName*\" }} | \
             Select-Object -First 500 | \
             Format-List TimeCreated, Id, LevelDisplayName, Message | Out-String",
            quote_powershell(&self.vm_name)
        );

        let output = self
            .runner
            .run(&powershell(script))?
            .into_success("failed to capture VM logs")?;

        Ok(output.stdout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_single_quotes_for_powershell() {
        assert_eq!(quote_powershell("it's"), "'it''s'");
    }

    #[test]
    fn builds_powershell_request_with_strict_errors() {
        let request = powershell("Get-VM");

        assert_eq!(request.program, "powershell");
        assert_eq!(
            request.args.last().map(String::as_str),
            Some("$ErrorActionPreference = 'Stop'; Get-VM")
        );
    }
}
