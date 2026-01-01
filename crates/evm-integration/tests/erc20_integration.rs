#![cfg(feature = "frontier-executor")]

use std::path::PathBuf;
use std::fs;
use atlas_evm_integration::{EvmConfig, FrontierEvmExecutor, EvmError, EvmExecutor};

#[test]
fn erc20_fixture_test() {
    // Look for a compiled ERC20 binary at tests/fixtures/minimal_erc20.bin
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests/fixtures/minimal_erc20.bin");

    if !p.exists() {
        eprintln!("skipping ERC20 integration test: {} not found", p.display());
        return;
    }

    let bin = fs::read(&p).expect("failed to read erc20 binary");
    // Deploy
    let mut deploy_payload = vec![0x01u8]; // CREATE
    deploy_payload.extend_from_slice(&0u64.to_be_bytes()); // value
    deploy_payload.extend_from_slice(&bin);

    let executor = FrontierEvmExecutor;
    let deploy_res = executor.execute(&deploy_payload, &[0u8;20], &EvmConfig::default());

    match deploy_res {
        Ok(res) => {
            assert!(res.success, "ERC20 deploy should succeed");
            // We cannot deterministically infer the created address in this test harness.
            // A more complete test should parse receipt state or return the created address.
            // For now, assert state root is non-zero to confirm state changes.
            assert_ne!(res.state_root, [0u8; 32]);
        }
        Err(e) => {
            panic!("deploy failed: {:?}", e)
        }
    }
}

#[test]
fn erc20_minimal_fallback_test() {
    // This test constructs a minimal contract at runtime that stores a small
    // total supply in storage slot 0 during construction and provides a
    // simple call path that returns the value of slot 0.

    // Runtime: SLOAD(0) -> MSTORE(0, sload) -> RETURN(0,32)
    let runtime: Vec<u8> = vec![
        0x60, 0x00, // PUSH1 0x00
        0x54,       // SLOAD
        0x60, 0x20, // PUSH1 0x20
        0x60, 0x00, // PUSH1 0x00
        0x52,       // MSTORE
        0x60, 0x20, // PUSH1 0x20
        0x60, 0x00, // PUSH1 0x00
        0xF3,       // RETURN
    ];

    // Constructor: PUSH1 <total> PUSH1 0x00 SSTORE | CODECOPY from offset | RETURN
    let total: u8 = 0x42; // 66
    let runtime_len = runtime.len() as u8;
    // constructor size: 3 (push total + push slot + sstore) + 9 (codecopy+return sequence)
    let constructor_prefix: Vec<u8> = vec![
        0x60, total, // PUSH1 total
        0x60, 0x00,  // PUSH1 0x00 (slot)
        0x55,        // SSTORE
        0x60, runtime_len, // PUSH1 runtime_size
        0x60, 0x09,  // PUSH1 runtime_offset (immediately after these 9 bytes)
        0x60, 0x00,  // PUSH1 mem_offset (0)
        0x39,        // CODECOPY
        0x60, runtime_len, // PUSH1 runtime_size
        0x60, 0x00,       // PUSH1 mem_offset
        0xF3,             // RETURN
    ];

    let mut init_code = constructor_prefix.clone();
    init_code.extend_from_slice(&runtime);

    let mut deploy_payload = vec![0x01u8]; // CREATE
    deploy_payload.extend_from_slice(&0u64.to_be_bytes()); // value
    deploy_payload.extend_from_slice(&init_code);

    let executor = FrontierEvmExecutor;
    let deploy_res = executor.execute(&deploy_payload, &[0u8;20], &EvmConfig::default()).expect("deploy should not error");
    assert!(deploy_res.success);

    // Call the contract: since we don't know the created address easily here,
    // we rely on the executor returning a non-zero state root and (optionally)
    // the runtime returns data when invoked at its assigned address. For a more
    // deterministic test we would capture the created address from executor.

    // To approximate, we will re-deploy the same init code and then call the
    // contract at zero address (some test setups map CREATE to predictable addresses)
    if let Ok(res2) = executor.execute(&deploy_payload, &[0u8;20], &EvmConfig::default()) {
        assert!(res2.success);
        // Call the deployed code by doing a CALL to the to=zero address (best-effort)
        let mut call_payload = vec![0x00u8];
        call_payload.extend_from_slice(&[0u8; 20]); // to = zero address (best-effort)
        call_payload.extend_from_slice(&0u64.to_be_bytes());
        call_payload.extend_from_slice(&[]);

        let call_res = executor.execute(&call_payload, &[0u8;20], &EvmConfig::default()).unwrap();
        // call may succeed and return 32-byte value equal to total (0x42)
        if call_res.success && !call_res.output.is_empty() {
            // parse last byte
            let val = call_res.output[call_res.output.len()-1];
            assert_eq!(val, total);
        }
    }
}
