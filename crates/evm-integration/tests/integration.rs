#![cfg(feature = "frontier-executor")]

use atlas_evm_integration::{EvmConfig, EvmExecutor, FrontierEvmExecutor};

#[test]
fn integration_state_root_and_logs() {
    let executor = FrontierEvmExecutor;

    // Contract 1: on creation store 0x42 at slot 0 and emit a LOG1
    let contract1 = vec![
        0x60, 0x42, // PUSH1 0x42
        0x60, 0x00, // PUSH1 0x00
        0x55, // SSTORE
        0x60, 0x20, // PUSH1 0x20 (size)
        0x60, 0x00, // PUSH1 0x00 (offset)
        0x60, 0x00, // PUSH1 0x00 (topic count)
        0xA1, // LOG1
        0x60, 0x00, // PUSH1 0x00
        0x60, 0x00, // PUSH1 0x00
        0xF3, // RETURN
    ];

    let mut payload = vec![0x01]; // CREATE
    payload.extend_from_slice(&0u64.to_be_bytes());
    payload.extend_from_slice(&contract1);

    let r1 = executor
        .execute(&payload, &[0xAAu8; 20], &EvmConfig::default())
        .unwrap();
    assert!(r1.success);
    assert_ne!(r1.state_root, [0u8; 32]);
    assert!(!r1.logs.is_empty());

    // Deploying an identical contract should produce a deterministic (but different)
    // state root when executed from the same caller/environment.
    let r2 = executor
        .execute(&payload, &[0xAAu8; 20], &EvmConfig::default())
        .unwrap();
    assert!(r2.success);

    // The two roots may differ (depending on address assignment), but both should be non-zero
    assert_ne!(r2.state_root, [0u8; 32]);
}
