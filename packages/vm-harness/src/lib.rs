use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

mod hyperv;

pub use hyperv::{
    CommandOutput, CommandRequest, CommandRunner, HyperVCreateOptions, HyperVProvider,
    SystemCommandRunner,
};

/// Virtualization backend used by the harness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Hypervisor {
    /// Hyper-V through the PowerShell module.
    HyperV,
    /// VirtualBox, reserved for a future backend.
    VirtualBox,
    /// VMware, reserved for a future backend.
    VMware,
    /// QEMU, reserved for a future backend.
    Qemu,
}

/// Configuration shared by the harness and its VM backends.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    /// VM name in the hypervisor.
    pub name: String,
    /// Backend that owns the VM.
    pub hypervisor: Hypervisor,
    /// Startup memory in megabytes.
    pub memory_mb: Option<u64>,
    /// Number of virtual processors.
    pub cpu_count: Option<u32>,
    /// Base disk used when the VM must be created.
    #[serde(default)]
    pub vhd_path: Option<String>,
    /// Virtual switch attached during creation.
    #[serde(default)]
    pub switch_name: Option<String>,
    /// Checkpoint used to restore the VM before and after a test.
    #[serde(default)]
    pub checkpoint_name: Option<String>,
    /// DPAPI credential file used by PowerShell Direct.
    #[serde(default)]
    pub credential_path: Option<String>,
    /// Hyper-V generation used during creation.
    #[serde(default)]
    pub generation: Option<u32>,
}

impl VmConfig {
    /// Builds a minimal configuration for `name`.
    pub fn new(name: impl Into<String>, hypervisor: Hypervisor) -> Self {
        Self {
            name: name.into(),
            hypervisor,
            memory_mb: None,
            cpu_count: None,
            vhd_path: None,
            switch_name: None,
            checkpoint_name: None,
            credential_path: None,
            generation: None,
        }
    }
}

/// High level harness that drives a VM backend.
pub struct VmHarness<R: CommandRunner> {
    config: VmConfig,
    provider: HyperVProvider<R>,
}

impl VmHarness<SystemCommandRunner> {
    /// Builds a harness that runs commands on the host.
    pub fn new(config: VmConfig) -> Self {
        Self::with_runner(config, SystemCommandRunner)
    }

    /// Detects the first supported hypervisor.
    pub fn detect_hypervisor() -> Result<Hypervisor> {
        let provider = HyperVProvider::new(SystemCommandRunner, "ll10-probe");

        if provider.detect()? {
            return Ok(Hypervisor::HyperV);
        }

        bail!("no supported hypervisor detected")
    }
}

impl<R: CommandRunner> VmHarness<R> {
    /// Builds a harness with a custom command runner.
    pub fn with_runner(config: VmConfig, runner: R) -> Self {
        let provider = HyperVProvider::new(runner, config.name.clone());

        Self { config, provider }
    }

    /// Returns the configuration used by this harness.
    pub fn config(&self) -> &VmConfig {
        &self.config
    }

    /// Returns the backend provider.
    pub fn provider(&self) -> &HyperVProvider<R> {
        &self.provider
    }

    /// Creates the VM when missing or restores its baseline checkpoint.
    pub fn create_or_restore(&self) -> Result<()> {
        tracing::info!(
            "Preparing VM {} with {:?}",
            self.config.name,
            self.config.hypervisor
        );

        match self.config.hypervisor {
            Hypervisor::HyperV => {
                if self.provider.exists()? {
                    let checkpoint = self
                        .config
                        .checkpoint_name
                        .as_deref()
                        .context("a checkpoint name is required to restore an existing VM")?;

                    self.provider.restore_checkpoint(checkpoint)
                } else {
                    self.provider.create(&self.create_options()?)
                }
            }
            _ => bail!("hypervisor not implemented"),
        }
    }

    /// Executes a command inside the guest.
    pub fn execute_command(&self, command: &str) -> Result<String> {
        tracing::info!("Executing command in VM {}: {}", self.config.name, command);

        match self.config.credential_path.as_deref() {
            Some(path) => self.provider.execute_command_with_credential(command, path),
            None => self.provider.execute_command(command),
        }
    }

    /// Captures VM logs from the host.
    pub fn capture_logs(&self) -> Result<String> {
        tracing::debug!("Capturing logs for VM {}", self.config.name);

        self.provider.capture_logs()
    }

    /// Restores the baseline checkpoint or removes the VM.
    pub fn revert_or_destroy(&self) -> Result<()> {
        tracing::info!("Reverting or destroying VM {}", self.config.name);

        if !self.provider.exists()? {
            return Ok(());
        }

        match self.config.checkpoint_name.as_deref() {
            Some(checkpoint) => self.provider.restore_checkpoint(checkpoint),
            None => self.provider.teardown(),
        }
    }

    /// Runs a reversible smoke test in the disposable VM.
    ///
    /// The VM is prepared, a harmless command runs, logs are captured, and the
    /// VM returns to its baseline checkpoint or is removed.
    pub fn run_smoke_test(&self) -> Result<bool> {
        tracing::info!("Running smoke test in VM {}", self.config.name);

        self.create_or_restore()?;

        let result = self.execute_command("Write-Output ll10-smoke");
        let logs = self.capture_logs();
        let cleanup = self.revert_or_destroy();

        result.context("smoke test command failed")?;
        logs.context("smoke test log capture failed")?;
        cleanup.context("smoke test cleanup failed")?;

        tracing::info!("Smoke test completed in isolated VM");
        Ok(true)
    }

    /// Builds creation options from the harness configuration.
    fn create_options(&self) -> Result<HyperVCreateOptions> {
        let vhd_path = self
            .config
            .vhd_path
            .clone()
            .context("a base disk path is required to create a VM")?;

        Ok(HyperVCreateOptions {
            memory_mb: self.config.memory_mb.unwrap_or(4096),
            cpu_count: self.config.cpu_count.unwrap_or(2),
            vhd_path,
            switch_name: self.config.switch_name.clone(),
            generation: self.config.generation.unwrap_or(2),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_minimal_vm_config() {
        let config = VmConfig::new("test-vm", Hypervisor::HyperV);

        assert_eq!(config.name, "test-vm");
        assert_eq!(config.hypervisor, Hypervisor::HyperV);
        assert_eq!(config.memory_mb, None);
        assert_eq!(config.vhd_path, None);
    }
}
