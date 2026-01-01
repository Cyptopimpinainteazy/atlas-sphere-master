#![cfg(feature = "rbpf-executor")]

use atlas_svm_integration::{SvmConfig, SvmExecutor, RbpfSvmExecutor};

/// Simple test: validate empty program fails
#[test]
fn test_svm_empty_program() {
    let executor = RbpfSvmExecutor::new();
    
    // Empty payload should fail validation
    let result = executor.validate_program(&[]);
    assert!(result.is_err(), "Empty program should fail validation");
}

/// Simple test: mock executor succeeds
#[test]
fn test_svm_mock_executor_basic() {
    use atlas_svm_integration::MockSvmExecutor;
    
    let executor = MockSvmExecutor;
    let result = executor.execute(
        &[0x01, 0x02],
        &[0u8; 32],
        &SvmConfig::default()
    );
    
    assert!(result.is_ok());
    let result = result.unwrap();
    assert!(result.success);
    assert_eq!(result.compute_units_used, SvmConfig::default().compute_unit_limit / 2);
}

/// Test: SVM config default values
#[test]
fn test_svm_config_defaults() {
    let config = SvmConfig::default();
    
    assert_eq!(config.compute_unit_limit, 200_000);
    assert_eq!(config.compute_unit_price, 1);
    assert_eq!(config.block_height, 0);
    assert_eq!(config.block_timestamp, 0);
    assert_eq!(config.cluster_id, 1);
}

/// Test: SVM config custom builder
#[test]
fn test_svm_config_custom() {
    let config = SvmConfig::new(
        500_000,   // compute_unit_limit
        5,         // compute_unit_price
        12345,     // block_height
        1234567890,// block_timestamp
        2,         // cluster_id
    );
    
    assert_eq!(config.compute_unit_limit, 500_000);
    assert_eq!(config.compute_unit_price, 5);
    assert_eq!(config.block_height, 12345);
    assert_eq!(config.block_timestamp, 1234567890);
    assert_eq!(config.cluster_id, 2);
}

/// Test: Mock executor with custom config
#[test]
fn test_svm_mock_custom_gas() {
    use atlas_svm_integration::MockSvmExecutor;
    
    let executor = MockSvmExecutor;
    let config = SvmConfig::new(
        100_000,
        2,
        100,
        1000,
        1,
    );
    
    let result = executor.execute(&[0x01], &[0u8; 32], &config).unwrap();
    
    assert!(result.success);
    // Mock uses half the gas limit
    assert_eq!(result.compute_units_used, 50_000);
}

/// Test: SVM execution result structure
#[test]
fn test_svm_execution_result_default() {
    use atlas_svm_integration::SvmExecutionResult;
    
    let result = SvmExecutionResult::default();
    
    assert!(!result.success);
    assert!(result.output.is_empty());
    assert_eq!(result.compute_units_used, 0);
    assert!(result.account_updates.is_empty());
    assert_eq!(result.state_root, [0u8; 32]);
    assert!(result.logs.is_empty());
}

/// Test: SVM error handling
#[test]
fn test_svm_mock_empty_payload() {
    use atlas_svm_integration::MockSvmExecutor;
    
    let executor = MockSvmExecutor;
    let result = executor.execute(&[], &[0u8; 32], &SvmConfig::default());
    
    assert!(result.is_err(), "Empty payload should return error");
}

/// Test: Multiple independent SVM executions
#[test]
fn test_svm_multiple_executions() {
    use atlas_svm_integration::MockSvmExecutor;
    
    let executor = MockSvmExecutor;
    let config = SvmConfig::default();
    
    let mut success_count = 0;
    for i in 0..5 {
        let payload = vec![i as u8; 10];
        let result = executor.execute(&payload, &[0u8; 32], &config);
        if result.is_ok() && result.unwrap().success {
            success_count += 1;
        }
    }
    
    assert_eq!(success_count, 5, "All executions should succeed");
}

/// Test: Different payers produce different contexts
#[test]
fn test_svm_different_payers() {
    use atlas_svm_integration::MockSvmExecutor;
    
    let executor = MockSvmExecutor;
    let config = SvmConfig::default();
    let payload = vec![0x01, 0x02, 0x03];
    
    let payer1 = [1u8; 32];
    let payer2 = [2u8; 32];
    
    let result1 = executor.execute(&payload, &payer1, &config).unwrap();
    let result2 = executor.execute(&payload, &payer2, &config).unwrap();
    
    // Both should succeed
    assert!(result1.success);
    assert!(result2.success);
    
    // Both should have same compute usage (mock is deterministic)
    assert_eq!(result1.compute_units_used, result2.compute_units_used);
}

/// Test: Account updates structure
#[test]
fn test_svm_account_update_structure() {
    use atlas_svm_integration::AccountUpdate;
    
    let update = AccountUpdate {
        pubkey: [1u8; 32],
        data: vec![0x01, 0x02, 0x03],
        lamports: 1_000_000,
        executable: false,
    };
    
    assert_eq!(update.pubkey, [1u8; 32]);
    assert_eq!(update.data, vec![0x01, 0x02, 0x03]);
    assert_eq!(update.lamports, 1_000_000);
    assert!(!update.executable);
}

/// Test: Compute unit pricing
#[test]
fn test_svm_compute_pricing() {
    let config = SvmConfig::new(
        200_000,
        10,      // 10 microlamports per compute unit
        0,
        0,
        1,
    );
    
    // At 200k compute units with 10 microlamports each
    // Total cost would be 2,000,000 microlamports (2 SOL)
    let expected_cost = (config.compute_unit_limit as u128) 
        * (config.compute_unit_price as u128);
    
    assert_eq!(expected_cost, 2_000_000);
}

/// Test: Block context in SVM config
#[test]
fn test_svm_block_context() {
    let config = SvmConfig::new(
        200_000,
        1,
        1_000_000,     // Block height
        1609459200,    // Block timestamp (2021-01-01 00:00:00 UTC)
        1,
    );
    
    assert_eq!(config.block_height, 1_000_000);
    assert_eq!(config.block_timestamp, 1609459200);
}

/// Test: Cluster identification
#[test]
fn test_svm_cluster_identification() {
    let mainnet = SvmConfig::new(200_000, 1, 0, 0, 1);  // Cluster 1
    let testnet = SvmConfig::new(200_000, 1, 0, 0, 2);  // Cluster 2
    let devnet = SvmConfig::new(200_000, 1, 0, 0, 3);   // Cluster 3
    
    assert_eq!(mainnet.cluster_id, 1);
    assert_eq!(testnet.cluster_id, 2);
    assert_eq!(devnet.cluster_id, 3);
}

/// Test: SVM payload validation success path
#[test]
fn test_svm_validation_success() {
    use atlas_svm_integration::MockSvmExecutor;
    
    let executor = MockSvmExecutor;
    let payload = vec![0x01, 0x02, 0x03, 0x04, 0x05];
    
    let result = executor.validate_program(&payload);
    assert!(result.is_ok(), "Valid non-empty payload should pass validation");
}

/// Test: State root computation
#[test]
fn test_svm_state_root_determinism() {
    use atlas_svm_integration::MockSvmExecutor;
    
    let executor = MockSvmExecutor;
    let config = SvmConfig::default();
    let payload = vec![0x01, 0x02];
    
    let result1 = executor.execute(&payload, &[0u8; 32], &config).unwrap();
    let result2 = executor.execute(&payload, &[0u8; 32], &config).unwrap();
    
    // State roots should be deterministic (even if empty)
    assert_eq!(result1.state_root, result2.state_root);
}

/// Test: Logs structure
#[test]
fn test_svm_logs_are_opaque() {
    use atlas_svm_integration::SvmExecutionResult;
    
    let result = SvmExecutionResult {
        success: true,
        output: vec![],
        compute_units_used: 5000,
        account_updates: vec![],
        state_root: [0u8; 32],
        logs: vec![
            vec![0x01, 0x02, 0x03],
            vec![0x04, 0x05],
        ],
    };
    
    assert_eq!(result.logs.len(), 2);
    assert_eq!(result.logs[0], vec![0x01, 0x02, 0x03]);
    assert_eq!(result.logs[1], vec![0x04, 0x05]);
}

/// Test: Maximum compute units
#[test]
fn test_svm_max_compute_units() {
    let config = SvmConfig::new(
        1_400_000,  // Maximum compute units per transaction on Solana
        1,
        0,
        0,
        1,
    );
    
    assert_eq!(config.compute_unit_limit, 1_400_000);
}

/// Test: Mock executor output format
#[test]
fn test_svm_mock_output() {
    use atlas_svm_integration::MockSvmExecutor;
    
    let executor = MockSvmExecutor;
    let result = executor.execute(&[0x01], &[0u8; 32], &SvmConfig::default()).unwrap();
    
    // Mock executor returns success indicator
    assert_eq!(result.output, vec![0x01]);
}

/// Test: Error variant coverage
#[test]
fn test_svm_error_variants() {
    use atlas_svm_integration::SvmError;
    
    let errors = vec![
        SvmError::InvalidPayload,
        SvmError::ExecutionFailed,
        SvmError::InvalidAccount,
        SvmError::InvalidSignature,
        SvmError::ExecutionError(1),
        SvmError::ExecutionError(255),
    ];
    
    assert_eq!(errors.len(), 6);
    
    // Verify they're distinct
    assert!(SvmError::InvalidPayload != SvmError::ExecutionFailed);
    assert!(SvmError::InvalidAccount != SvmError::InvalidSignature);
    assert!(SvmError::ExecutionError(1) != SvmError::ExecutionError(255));
}
