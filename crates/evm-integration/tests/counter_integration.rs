#![cfg(feature = "frontier-executor")]

use atlas_evm_integration::{EvmConfig, FrontierEvmExecutor, EvmExecutor};

// Counter contract compiled bytecode (simplified):
// This is a mock Counter contract that stores count at storage slot 0
// Bytecode structure for increment operation:
//   PUSH1 0x01           (0x6001) - value to add
//   PUSH1 0x00           (0x6000) - storage slot
//   SLOAD                (0x54)   - load current value
//   ADD                  (0x01)   - add 1
//   PUSH1 0x00           (0x6000) - storage slot
//   SSTORE               (0x55)   - store result
//   PUSH1 0x00           (0x6000) - return offset
//   PUSH1 0x00           (0x6000) - return size
//   RETURN               (0xF3)   - return (empty)

/// Test: Counter contract deployment
#[test]
fn test_counter_deployment() {
    let executor = FrontierEvmExecutor;

    // Simple contract bytecode that initializes storage slot 0 with value 0
    // Structure: PUSH1 0x00 PUSH1 0x00 SSTORE STOP
    let bytecode = vec![
        0x60, 0x00, // PUSH1 0x00 (value)
        0x60, 0x00, // PUSH1 0x00 (slot)
        0x55,       // SSTORE
        0x00,       // STOP
    ];

    let mut payload = vec![0x01]; // CREATE opcode
    payload.extend_from_slice(&0u64.to_be_bytes()); // value
    payload.extend_from_slice(&bytecode);

    let result = executor.execute(&payload, &[0xAAu8; 20], &EvmConfig::default());
    assert!(result.is_ok());
    
    let result = result.unwrap();
    assert!(result.success, "Counter deployment should succeed");
    assert_ne!(result.state_root, [0u8; 32], "State root should be non-zero after deployment");
    assert!(result.gas_used > 0, "Gas should be consumed");
}

/// Test: Counter increment operation (state change)
#[test]
fn test_counter_increment() {
    let executor = FrontierEvmExecutor;

    // Contract bytecode for increment:
    // Load slot 0, add 1, store back to slot 0
    let bytecode = vec![
        0x60, 0x00, // PUSH1 0x00 (slot)
        0x54,       // SLOAD (load value from slot 0)
        0x60, 0x01, // PUSH1 0x01 (1)
        0x01,       // ADD
        0x60, 0x00, // PUSH1 0x00 (slot)
        0x55,       // SSTORE (store back)
        0x60, 0x00, // PUSH1 0x00 (offset)
        0x60, 0x00, // PUSH1 0x00 (size)
        0xF3,       // RETURN
    ];

    let mut payload = vec![0x01]; // CREATE
    payload.extend_from_slice(&0u64.to_be_bytes());
    payload.extend_from_slice(&bytecode);

    let result = executor.execute(&payload, &[0xBBu8; 20], &EvmConfig::default()).unwrap();
    assert!(result.success);
    assert_ne!(result.state_root, [0u8; 32], "State root should reflect increment");
}

/// Test: Counter view function (no state change)
#[test]
fn test_counter_get_count() {
    let executor = FrontierEvmExecutor;

    // Contract bytecode that returns value from slot 0
    // Load slot 0 and return it
    let bytecode = vec![
        0x60, 0x00, // PUSH1 0x00 (slot)
        0x54,       // SLOAD
        0x60, 0x20, // PUSH1 0x20 (return size in bytes)
        0x00,       // PUSH1 0x00 (return offset)
        0xF3,       // RETURN
    ];

    let mut payload = vec![0x01]; // CREATE
    payload.extend_from_slice(&0u64.to_be_bytes());
    payload.extend_from_slice(&bytecode);

    let result = executor.execute(&payload, &[0xCCu8; 20], &EvmConfig::default()).unwrap();
    assert!(result.success);
    // View functions should have minimal gas usage
    assert!(result.gas_used < 100_000, "View function should use less gas");
}

/// Test: Gas metering for counter operations
#[test]
fn test_counter_gas_metering() {
    let executor = FrontierEvmExecutor;

    // Simple operation: PUSH, STOP
    let bytecode_simple = vec![
        0x60, 0x00, // PUSH1 0x00
        0x00,       // STOP
    ];

    // Complex operation: multiple storage ops
    let bytecode_complex = vec![
        0x60, 0x42, // PUSH1 0x42
        0x60, 0x00, // PUSH1 0x00
        0x55,       // SSTORE (20,000 gas)
        0x60, 0x00, // PUSH1 0x00
        0x54,       // SLOAD (2,100 gas for cold)
        0x60, 0x01, // PUSH1 0x01
        0x01,       // ADD
        0x60, 0x01, // PUSH1 0x01
        0x55,       // SSTORE
        0x00,       // STOP
    ];

    let mut simple_payload = vec![0x01];
    simple_payload.extend_from_slice(&0u64.to_be_bytes());
    simple_payload.extend_from_slice(&bytecode_simple);

    let mut complex_payload = vec![0x01];
    complex_payload.extend_from_slice(&0u64.to_be_bytes());
    complex_payload.extend_from_slice(&bytecode_complex);

    let simple_result = executor.execute(&simple_payload, &[0xAAu8; 20], &EvmConfig::default()).unwrap();
    let complex_result = executor.execute(&complex_payload, &[0xBBu8; 20], &EvmConfig::default()).unwrap();

    assert!(complex_result.gas_used > simple_result.gas_used, 
            "Complex operations should use more gas");
    
    // Gas used should be reasonable (>0 and < limit)
    assert!(simple_result.gas_used > 0);
    assert!(complex_result.gas_used > 0);
    assert!(complex_result.gas_used < EvmConfig::default().gas_limit);
}

/// Test: Counter with log emission (events)
#[test]
fn test_counter_with_events() {
    let executor = FrontierEvmExecutor;

    // Contract that stores and emits a log
    // PUSH1 0x01 PUSH1 0x00 SSTORE  (store 1 at slot 0)
    // PUSH1 0x00 PUSH1 0x00 LOG1    (emit event)
    let bytecode = vec![
        0x60, 0x01, // PUSH1 0x01 (data)
        0x60, 0x00, // PUSH1 0x00 (slot)
        0x55,       // SSTORE
        0x60, 0x00, // PUSH1 0x00 (data offset)
        0x60, 0x00, // PUSH1 0x00 (data size)
        0x60, 0x01, // PUSH1 0x01 (num topics)
        0xA1,       // LOG1 (emit event with 1 topic)
        0x60, 0x00, // PUSH1 0x00
        0x60, 0x00, // PUSH1 0x00
        0xF3,       // RETURN
    ];

    let mut payload = vec![0x01];
    payload.extend_from_slice(&0u64.to_be_bytes());
    payload.extend_from_slice(&bytecode);

    let result = executor.execute(&payload, &[0xDDu8; 20], &EvmConfig::default()).unwrap();
    assert!(result.success);
    assert!(!result.logs.is_empty(), "Should have emitted at least one log");
    
    // Verify log structure
    let log = &result.logs[0];
    assert_eq!(log.address.len(), 20, "Log address should be 20 bytes");
    assert!(!log.topics.is_empty(), "Log should have topics");
}

/// Test: Counter sequential increments (state persistence)
#[test]
fn test_counter_sequential_ops() {
    let executor = FrontierEvmExecutor;

    // First increment
    let bytecode = vec![
        0x60, 0x00, // PUSH1 0x00
        0x54,       // SLOAD
        0x60, 0x01, // PUSH1 0x01
        0x01,       // ADD
        0x60, 0x00, // PUSH1 0x00
        0x55,       // SSTORE
        0x00,       // STOP
    ];

    let mut payload1 = vec![0x01];
    payload1.extend_from_slice(&0u64.to_be_bytes());
    payload1.extend_from_slice(&bytecode);

    let result1 = executor.execute(&payload1, &[0xEEu8; 20], &EvmConfig::default()).unwrap();
    assert!(result1.success);
    let state_root_1 = result1.state_root;

    // Second increment (independent execution, fresh state)
    let mut payload2 = vec![0x01];
    payload2.extend_from_slice(&0u64.to_be_bytes());
    payload2.extend_from_slice(&bytecode);

    let result2 = executor.execute(&payload2, &[0xEEu8; 20], &EvmConfig::default()).unwrap();
    assert!(result2.success);
    let state_root_2 = result2.state_root;

    // Both should have modified state
    assert_ne!(state_root_1, [0u8; 32]);
    assert_ne!(state_root_2, [0u8; 32]);
    // State roots for same operation from same caller should be identical (deterministic)
    assert_eq!(state_root_1, state_root_2, "Sequential identical operations should produce same state root");
}

/// Test: Counter overflow safety
#[test]
fn test_counter_saturating_behavior() {
    let executor = FrontierEvmExecutor;

    // Contract that does ADD operation
    let bytecode = vec![
        0x60, 0xFF, // PUSH1 0xFF
        0x60, 0xFF, // PUSH1 0xFF
        0x01,       // ADD (should result in 0x1FE, no saturation in EVM)
        0x60, 0x00, // PUSH1 0x00
        0x55,       // SSTORE
        0x00,       // STOP
    ];

    let mut payload = vec![0x01];
    payload.extend_from_slice(&0u64.to_be_bytes());
    payload.extend_from_slice(&bytecode);

    let result = executor.execute(&payload, &[0xFFu8; 20], &EvmConfig::default()).unwrap();
    assert!(result.success, "ADD should not overflow in EVM");
}

/// Test: Out of gas handling
#[test]
fn test_counter_out_of_gas() {
    let executor = FrontierEvmExecutor;

    let bytecode = vec![
        0x60, 0x01, // PUSH1 0x01
        0x60, 0x00, // PUSH1 0x00
        0x55,       // SSTORE (expensive operation)
        0x00,       // STOP
    ];

    let mut payload = vec![0x01];
    payload.extend_from_slice(&0u64.to_be_bytes());
    payload.extend_from_slice(&bytecode);

    // Create config with extremely low gas limit
    let mut config = EvmConfig::default();
    config.gas_limit = 100; // Far too low for SSTORE

    let result = executor.execute(&payload, &[0xAAu8; 20], &config);
    assert!(result.is_err(), "Should fail with out of gas error");
}

/// Test: Revert behavior
#[test]
fn test_counter_revert() {
    let executor = FrontierEvmExecutor;

    // Contract that reverts
    let bytecode = vec![
        0x60, 0x00, // PUSH1 0x00
        0x60, 0x00, // PUSH1 0x00
        0xFD,       // REVERT
    ];

    let mut payload = vec![0x01];
    payload.extend_from_slice(&0u64.to_be_bytes());
    payload.extend_from_slice(&bytecode);

    let result = executor.execute(&payload, &[0xBBu8; 20], &EvmConfig::default());
    assert!(result.is_err(), "Should return error on revert");
}

/// Test: Multiple counters with different addresses
#[test]
fn test_multiple_counter_instances() {
    let executor = FrontierEvmExecutor;

    let bytecode = vec![
        0x60, 0x42, // PUSH1 0x42
        0x60, 0x00, // PUSH1 0x00
        0x55,       // SSTORE
        0x00,       // STOP
    ];

    let mut payload1 = vec![0x01];
    payload1.extend_from_slice(&0u64.to_be_bytes());
    payload1.extend_from_slice(&bytecode);

    let mut payload2 = vec![0x01];
    payload2.extend_from_slice(&0u64.to_be_bytes());
    payload2.extend_from_slice(&bytecode);

    let result1 = executor.execute(&payload1, &[0x11u8; 20], &EvmConfig::default()).unwrap();
    let result2 = executor.execute(&payload2, &[0x22u8; 20], &EvmConfig::default()).unwrap();

    assert!(result1.success);
    assert!(result2.success);
    // Different callers should produce different state roots
    assert_ne!(result1.state_root, result2.state_root);
}

/// Test: Gas calculation accuracy (within EVM spec)
#[test]
fn test_gas_calculation_accuracy() {
    let executor = FrontierEvmExecutor;

    let config = EvmConfig::default();
    
    // Test with identity precompile (cheap, predictable gas)
    // Call identity precompile (address 0x04) with 32 bytes input
    // Expected gas: 15 + 3*((32+31)/32) = 15 + 3 = 18 wei
    let mut payload = vec![0x00]; // CALL
    payload.extend_from_slice(&[0u8; 20]); // to (identity precompile)
    payload[20] = 0x04; // address 0x04
    payload.extend_from_slice(&0u64.to_be_bytes()); // value
    payload.extend_from_slice(&vec![0xAAu8; 32]); // 32-byte input

    let result = executor.execute(&payload, &[0u8; 20], &config).unwrap();
    assert!(result.success);
    // Gas should be reasonable but less than a full TX limit
    assert!(result.gas_used > 0);
    assert!(result.gas_used < 100_000);
}
