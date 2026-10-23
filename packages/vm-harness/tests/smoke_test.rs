use vm_harness::{Hypervisor, VmConfig, VmHarness};

#[test]
fn test_reversible_smoke_test() {
    let config = VmConfig {
        name: "smoke-test-vm".to_string(),
        hypervisor: Hypervisor::HyperV,
        memory_mb: Some(2048),
        cpu_count: Some(2),
    };
    
    let harness = VmHarness::new(config);
    
    // This demonstrates the reversible flow
    // 1. Create or restore VM (from known state)
    // 2. Execute safe command
    // 3. Capture logs
    // 4. Revert or destroy VM
    let result = harness.run_smoke_test();
    assert!(result.is_ok());
}
