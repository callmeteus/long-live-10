use anyhow::Result;
use std::collections::VecDeque;
use std::sync::Mutex;
use vm_harness::{
    CommandOutput, CommandRequest, CommandRunner, HyperVCreateOptions, HyperVProvider, Hypervisor,
    VmConfig, VmHarness,
};

/// Records requests and replays queued outputs in order.
#[derive(Default)]
struct FakeRunner {
    requests: Mutex<Vec<CommandRequest>>,
    responses: Mutex<VecDeque<CommandOutput>>,
}

impl FakeRunner {
    fn queue(&self, output: CommandOutput) {
        self.responses
            .lock()
            .expect("response lock")
            .push_back(output);
    }

    fn succeed(&self, stdout: impl Into<String>) {
        self.queue(CommandOutput {
            success: true,
            exit_code: Some(0),
            stdout: stdout.into(),
            stderr: String::new(),
        });
    }

    fn fail(&self, stderr: impl Into<String>) {
        self.queue(CommandOutput {
            success: false,
            exit_code: Some(1),
            stdout: String::new(),
            stderr: stderr.into(),
        });
    }

    fn requests(&self) -> Vec<CommandRequest> {
        self.requests.lock().expect("request lock").clone()
    }

    fn scripts(&self) -> Vec<String> {
        self.requests()
            .into_iter()
            .map(|request| request.args.last().cloned().unwrap_or_default())
            .collect()
    }
}

impl CommandRunner for &FakeRunner {
    fn run(&self, request: &CommandRequest) -> Result<CommandOutput> {
        self.requests
            .lock()
            .expect("request lock")
            .push(request.clone());

        let output = self
            .responses
            .lock()
            .expect("response lock")
            .pop_front()
            .expect("unexpected host command");

        Ok(output)
    }
}

fn new_runner() -> FakeRunner {
    FakeRunner::default()
}

fn provider<'a>(runner: &'a FakeRunner, name: &str) -> HyperVProvider<&'a FakeRunner> {
    HyperVProvider::new(runner, name)
}

fn create_options() -> HyperVCreateOptions {
    HyperVCreateOptions {
        memory_mb: 2048,
        cpu_count: 2,
        vhd_path: "C:\\VMs\\base.vhdx".to_string(),
        switch_name: Some("Default Switch".to_string()),
        generation: 2,
    }
}

#[test]
fn detects_the_hyper_v_module() {
    let runner = new_runner();
    runner.succeed("");

    let hyperv = provider(&runner, "ll10-test");

    assert!(hyperv.detect().expect("detect"));
    assert_eq!(
        runner.scripts(),
        vec!["$ErrorActionPreference = 'Stop'; Get-Command Get-VM | Out-Null"]
    );
}

#[test]
fn reports_a_missing_hyper_v_module() {
    let runner = new_runner();
    runner.fail("Get-VM is not recognized");

    let hyperv = provider(&runner, "ll10-test");

    assert!(!hyperv.detect().expect("detect"));
}

#[test]
fn checks_whether_the_vm_exists() {
    let runner = new_runner();
    runner.succeed("");

    let hyperv = provider(&runner, "ll10-test");

    assert!(hyperv.exists().expect("exists"));
    assert_eq!(
        runner.scripts(),
        vec!["$ErrorActionPreference = 'Stop'; Get-VM -Name 'll10-test' | Out-Null"]
    );
}

#[test]
fn treats_a_failed_lookup_as_a_missing_vm() {
    let runner = new_runner();
    runner.fail("A virtual machine with name 'll10-test' was not found.");

    let hyperv = provider(&runner, "ll10-test");

    assert!(!hyperv.exists().expect("exists"));
}

#[test]
fn creates_and_starts_a_vm() {
    let runner = new_runner();
    runner.succeed("");
    runner.succeed("");
    runner.succeed("");

    let hyperv = provider(&runner, "ll10-test");

    hyperv.create(&create_options()).expect("create");

    assert_eq!(
        runner.scripts(),
        vec![
            "$ErrorActionPreference = 'Stop'; New-VM -Name 'll10-test' \
             -MemoryStartupBytes 2147483648 -Generation 2 \
             -VHDPath 'C:\\VMs\\base.vhdx' -SwitchName 'Default Switch'",
            "$ErrorActionPreference = 'Stop'; Set-VMProcessor -VMName 'll10-test' -Count 2",
            "$ErrorActionPreference = 'Stop'; Start-VM -Name 'll10-test'",
        ]
    );
}

#[test]
fn creates_a_vm_without_a_switch() {
    let runner = new_runner();
    runner.succeed("");
    runner.succeed("");
    runner.succeed("");

    let hyperv = provider(&runner, "ll10-test");

    let mut options = create_options();
    options.switch_name = None;

    hyperv.create(&options).expect("create");

    let scripts = runner.scripts();

    assert!(scripts[0].contains("-VHDPath 'C:\\VMs\\base.vhdx'"));
    assert!(!scripts[0].contains("-SwitchName"));
}

#[test]
fn rejects_invalid_create_options() {
    let runner = new_runner();
    let hyperv = provider(&runner, "ll10-test");

    let mut options = create_options();
    options.memory_mb = 0;

    assert!(hyperv.create(&options).is_err());

    let mut options = create_options();
    options.cpu_count = 0;

    assert!(hyperv.create(&options).is_err());

    let mut options = create_options();
    options.vhd_path = "   ".to_string();

    assert!(hyperv.create(&options).is_err());
    assert!(runner.requests().is_empty());
}

#[test]
fn propagates_creation_failures() {
    let runner = new_runner();
    runner.fail("Hyper-V could not create the VM");

    let hyperv = provider(&runner, "ll10-test");

    let error = hyperv
        .create(&create_options())
        .expect_err("create must fail");

    assert!(error.to_string().contains("failed to create VM"));
    assert!(error
        .to_string()
        .contains("Hyper-V could not create the VM"));
}

#[test]
fn restores_a_checkpoint_and_starts_the_vm() {
    let runner = new_runner();
    runner.succeed("");
    runner.succeed("");

    let hyperv = provider(&runner, "ll10-test");

    hyperv.restore_checkpoint("baseline").expect("restore");

    assert_eq!(
        runner.scripts(),
        vec![
            "$ErrorActionPreference = 'Stop'; Restore-VMSnapshot -VMName 'll10-test' \
             -Name 'baseline' -Confirm:$false",
            "$ErrorActionPreference = 'Stop'; Start-VM -Name 'll10-test'",
        ]
    );
}

#[test]
fn rejects_an_empty_checkpoint_name() {
    let runner = new_runner();
    let hyperv = provider(&runner, "ll10-test");

    assert!(hyperv.restore_checkpoint("  ").is_err());
    assert!(runner.requests().is_empty());
}

#[test]
fn executes_a_guest_command_and_returns_output() {
    let runner = new_runner();
    runner.succeed("hello\r\n");

    let hyperv = provider(&runner, "ll10-test");

    let output = hyperv.execute_command("echo 'hello'").expect("execute");

    assert_eq!(output, "hello");

    let scripts = runner.scripts();

    assert!(scripts[0].contains("-ScriptBlock { param($command) Invoke-Expression $command }"));
    assert!(scripts[0].contains("-ArgumentList 'echo ''hello'''"));
}

#[test]
fn returns_guest_command_failures() {
    let runner = new_runner();
    runner.fail("Access is denied.");

    let hyperv = provider(&runner, "ll10-test");

    let error = hyperv
        .execute_command("whoami")
        .expect_err("execute must fail");

    assert!(error.to_string().contains("guest command failed"));
    assert!(error.to_string().contains("Access is denied."));
}

#[test]
fn executes_a_guest_command_with_a_credential_file() {
    let runner = new_runner();
    runner.succeed("hello\r\n");

    let hyperv = provider(&runner, "ll10-test");

    let output = hyperv
        .execute_command_with_credential("whoami", "C:\\ll10\\guest.xml")
        .expect("execute");

    assert_eq!(output, "hello");

    let scripts = runner.scripts();

    assert!(scripts[0].contains("$credential = Import-Clixml 'C:\\ll10\\guest.xml';"));
    assert!(scripts[0].contains("-Credential $credential"));
}

#[test]
fn rejects_an_empty_credential_path() {
    let runner = new_runner();
    let hyperv = provider(&runner, "ll10-test");

    assert!(hyperv
        .execute_command_with_credential("whoami", "  ")
        .is_err());
    assert!(runner.requests().is_empty());
}

#[test]
fn rejects_an_empty_guest_command() {
    let runner = new_runner();
    let hyperv = provider(&runner, "ll10-test");

    assert!(hyperv.execute_command("  ").is_err());
    assert!(runner.requests().is_empty());
}

#[test]
fn captures_hyper_v_worker_logs() {
    let runner = new_runner();
    runner.succeed("TimeCreated: today\nMessage: started");

    let hyperv = provider(&runner, "ll10-test");

    let logs = hyperv.capture_logs().expect("capture logs");

    assert!(logs.contains("TimeCreated"));

    let scripts = runner.scripts();

    assert!(scripts[0].contains("$vmName = 'll10-test'"));
    assert!(scripts[0].contains("Microsoft-Windows-Hyper-V-VMMS-Admin"));
    assert!(scripts[0].contains("Where-Object"));
}

#[test]
fn stops_and_tears_down_a_vm() {
    let runner = new_runner();
    runner.succeed("");
    runner.succeed("");

    let hyperv = provider(&runner, "ll10-test");

    hyperv.stop().expect("stop");
    hyperv.teardown().expect("teardown");

    assert_eq!(
        runner.scripts(),
        vec![
            "$ErrorActionPreference = 'Stop'; Stop-VM -Name 'll10-test' -TurnOff -Force",
            "$ErrorActionPreference = 'Stop'; Stop-VM -Name 'll10-test' -TurnOff -Force \
             -ErrorAction SilentlyContinue; Remove-VM -Name 'll10-test' -Force",
        ]
    );
}

#[test]
fn runs_a_reversible_smoke_test() {
    let runner = new_runner();

    runner.fail("A virtual machine with name 'll10-test' was not found.");
    runner.succeed("");
    runner.succeed("");
    runner.succeed("");
    runner.succeed("ll10-smoke\r\n");
    runner.succeed("TimeCreated: today");
    runner.succeed("");
    runner.succeed("");
    runner.succeed("");

    let mut config = VmConfig::new("ll10-test", Hypervisor::HyperV);
    config.memory_mb = Some(2048);
    config.cpu_count = Some(2);
    config.vhd_path = Some("C:\\VMs\\base.vhdx".to_string());
    config.checkpoint_name = Some("baseline".to_string());

    let harness = VmHarness::with_runner(config, &runner);

    assert!(harness.run_smoke_test().expect("smoke test"));

    let scripts = runner.scripts();

    assert_eq!(scripts.len(), 9);
    assert!(scripts[0].contains("Get-VM -Name 'll10-test'"));
    assert!(scripts[1].contains("New-VM"));
    assert!(scripts[4].contains("Write-Output ll10-smoke"));
    assert!(scripts[8].contains("Start-VM -Name 'll10-test'"));
}

#[test]
fn harness_requires_a_base_disk_to_create_a_vm() {
    let runner = new_runner();
    runner.fail("not found");

    let config = VmConfig::new("ll10-test", Hypervisor::HyperV);
    let harness = VmHarness::with_runner(config, &runner);

    let error = harness.create_or_restore().expect_err("create must fail");

    assert!(error.to_string().contains("base disk path"));
}
