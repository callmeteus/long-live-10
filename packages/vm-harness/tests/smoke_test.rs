use vm_harness::{Hypervisor, VmConfig, VmHarness};

/// End-to-end smoke test against a prepared disposable Hyper-V VM.
///
/// Enable it only after the base image from `docs/` is available:
///
/// ```powershell
/// $env:LL10_VM_NAME = "ll10-22h2-base"
/// $env:LL10_VM_VHD = "C:\ll10\base\Windows10-22H2.vhdx"
/// $env:LL10_VM_CHECKPOINT = "baseline"
/// cargo test -p vm-harness --test smoke_test -- --ignored
/// ```
#[test]
#[ignore = "requires a prepared Hyper-V base image and checkpoint"]
fn test_reversible_smoke_test() {
    let name = std::env::var("LL10_VM_NAME").expect("LL10_VM_NAME must be set");
    let vhd_path = std::env::var("LL10_VM_VHD").expect("LL10_VM_VHD must be set");
    let checkpoint_name =
        std::env::var("LL10_VM_CHECKPOINT").expect("LL10_VM_CHECKPOINT must be set");

    let mut config = VmConfig::new(name, Hypervisor::HyperV);
    config.memory_mb = Some(4096);
    config.cpu_count = Some(2);
    config.vhd_path = Some(vhd_path);
    config.checkpoint_name = Some(checkpoint_name);
    config.credential_path = std::env::var("LL10_VM_CREDENTIAL").ok();

    let harness = VmHarness::new(config);

    let result = harness.run_smoke_test();

    assert!(result.is_ok());
}
