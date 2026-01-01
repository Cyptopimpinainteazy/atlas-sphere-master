//! AMM Adapter Module
//!
//! Provides unified interfaces for interacting with various AMM protocols:
//! - UniswapV2 (EVM) - Constant product AMM
//! - UniswapV3 (EVM) - Concentrated liquidity AMM
//! - Raydium (SVM) - Solana AMM
//! - Orca Whirlpool (SVM) - Concentrated liquidity on Solana
//! - Atlas AMM - Native cross-VM AMM

use crate::types::{AmmProtocol, VmType};
use codec::{Decode, Encode};
use frame_support::pallet_prelude::*;
use scale_info::TypeInfo;
use sp_core::H256;
use sp_runtime::RuntimeDebug;
use sp_std::vec::Vec;

/// Swap parameters for AMM execution
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct SwapParams {
    /// Input token
    pub token_in: H256,
    /// Output token
    pub token_out: H256,
    /// Amount of input token
    pub amount_in: u128,
    /// Minimum acceptable output
    pub min_amount_out: u128,
    /// Recipient address (VM-specific encoding)
    pub recipient: Vec<u8>,
    /// Deadline (block number or timestamp)
    pub deadline: u64,
    /// Additional protocol-specific data
    pub extra_data: Vec<u8>,
}

/// Result of a swap execution
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct SwapResult {
    /// Actual output amount
    pub amount_out: u128,
    /// Gas/compute units used
    pub gas_used: u64,
    /// Price impact in basis points
    pub price_impact_bps: u32,
    /// Execution logs
    pub logs: Vec<Vec<u8>>,
}

/// Trait for AMM adapters
pub trait AmmAdapter {
    /// Get the protocol this adapter handles
    fn protocol() -> AmmProtocol;

    /// Get the VM type
    fn vm_type() -> VmType;

    /// Build swap calldata for the AMM
    fn build_swap_calldata(params: &SwapParams) -> Result<Vec<u8>, DispatchError>;

    /// Parse swap result from execution receipt
    fn parse_swap_result(return_data: &[u8], gas_used: u64) -> Result<SwapResult, DispatchError>;

    /// Get quote for a swap (read-only)
    fn get_quote(
        token_in: H256,
        token_out: H256,
        amount_in: u128,
        pool_address: &[u8],
    ) -> Result<u128, DispatchError>;

    /// Calculate price impact
    fn calculate_price_impact(
        amount_in: u128,
        amount_out: u128,
        reserve_in: u128,
        reserve_out: u128,
    ) -> u32;
}

// ============================================================================
// UniswapV2 Adapter (EVM)
// ============================================================================

/// UniswapV2-style constant product AMM adapter
pub struct UniswapV2Adapter;

impl AmmAdapter for UniswapV2Adapter {
    fn protocol() -> AmmProtocol {
        AmmProtocol::UniswapV2
    }

    fn vm_type() -> VmType {
        VmType::Evm
    }

    fn build_swap_calldata(params: &SwapParams) -> Result<Vec<u8>, DispatchError> {
        let mut calldata = Vec::new();

        // Function selector: swapExactTokensForTokens
        calldata.extend_from_slice(&[0x38, 0xed, 0x17, 0x39]);

        // amount_in (uint256)
        calldata.extend_from_slice(&encode_uint256(params.amount_in));

        // amount_out_min (uint256)
        calldata.extend_from_slice(&encode_uint256(params.min_amount_out));

        // path offset (uint256)
        calldata.extend_from_slice(&encode_uint256(160));

        // to (address)
        let mut to_padded = [0u8; 32];
        let to_len = params.recipient.len().min(20);
        to_padded[32 - to_len..].copy_from_slice(&params.recipient[..to_len]);
        calldata.extend_from_slice(&to_padded);

        // deadline (uint256)
        calldata.extend_from_slice(&encode_uint256(params.deadline as u128));

        // path array length
        calldata.extend_from_slice(&encode_uint256(2));

        // path[0] = token_in
        calldata.extend_from_slice(params.token_in.as_bytes());

        // path[1] = token_out
        calldata.extend_from_slice(params.token_out.as_bytes());

        Ok(calldata)
    }

    fn parse_swap_result(return_data: &[u8], gas_used: u64) -> Result<SwapResult, DispatchError> {
        let amount_out = if return_data.len() >= 64 {
            let offset = return_data.len() - 32;
            decode_uint256(&return_data[offset..])?
        } else {
            return Err(DispatchError::Other("Invalid return data"));
        };

        Ok(SwapResult {
            amount_out,
            gas_used,
            price_impact_bps: 0,
            logs: Vec::new(),
        })
    }

    fn get_quote(
        _token_in: H256,
        _token_out: H256,
        amount_in: u128,
        _pool_address: &[u8],
    ) -> Result<u128, DispatchError> {
        let fee_adjusted = amount_in.saturating_mul(997) / 1000;
        Ok(fee_adjusted)
    }

    fn calculate_price_impact(
        amount_in: u128,
        amount_out: u128,
        reserve_in: u128,
        reserve_out: u128,
    ) -> u32 {
        if reserve_in == 0 || amount_in == 0 {
            return 0;
        }

        let spot_scaled = reserve_out.saturating_mul(1_000_000) / reserve_in;
        let exec_scaled = amount_out.saturating_mul(1_000_000) / amount_in;

        if exec_scaled >= spot_scaled {
            return 0;
        }

        let impact = spot_scaled
            .saturating_sub(exec_scaled)
            .saturating_mul(10000)
            / spot_scaled;

        impact as u32
    }
}

// ============================================================================
// UniswapV3 Adapter (EVM)
// ============================================================================

/// UniswapV3 concentrated liquidity AMM adapter
pub struct UniswapV3Adapter;

impl AmmAdapter for UniswapV3Adapter {
    fn protocol() -> AmmProtocol {
        AmmProtocol::UniswapV3
    }

    fn vm_type() -> VmType {
        VmType::Evm
    }

    fn build_swap_calldata(params: &SwapParams) -> Result<Vec<u8>, DispatchError> {
        let mut calldata = Vec::new();

        // Function selector: exactInputSingle
        calldata.extend_from_slice(&[0x41, 0x4b, 0xf3, 0x89]);

        // tokenIn (address)
        let mut token_in_padded = [0u8; 32];
        token_in_padded[12..].copy_from_slice(&params.token_in.as_bytes()[..20]);
        calldata.extend_from_slice(&token_in_padded);

        // tokenOut (address)
        let mut token_out_padded = [0u8; 32];
        token_out_padded[12..].copy_from_slice(&params.token_out.as_bytes()[..20]);
        calldata.extend_from_slice(&token_out_padded);

        // fee (uint24) - default 3000
        let fee: u32 = if params.extra_data.len() >= 4 {
            u32::from_be_bytes([
                params.extra_data[0],
                params.extra_data[1],
                params.extra_data[2],
                params.extra_data[3],
            ])
        } else {
            3000
        };
        calldata.extend_from_slice(&encode_uint256(fee as u128));

        // recipient (address)
        let mut recipient_padded = [0u8; 32];
        let recip_len = params.recipient.len().min(20);
        recipient_padded[32 - recip_len..].copy_from_slice(&params.recipient[..recip_len]);
        calldata.extend_from_slice(&recipient_padded);

        // deadline (uint256)
        calldata.extend_from_slice(&encode_uint256(params.deadline as u128));

        // amountIn (uint256)
        calldata.extend_from_slice(&encode_uint256(params.amount_in));

        // amountOutMinimum (uint256)
        calldata.extend_from_slice(&encode_uint256(params.min_amount_out));

        // sqrtPriceLimitX96 (uint160) - 0 means no limit
        calldata.extend_from_slice(&[0u8; 32]);

        Ok(calldata)
    }

    fn parse_swap_result(return_data: &[u8], gas_used: u64) -> Result<SwapResult, DispatchError> {
        let amount_out = if return_data.len() >= 32 {
            decode_uint256(&return_data[..32])?
        } else {
            return Err(DispatchError::Other("Invalid return data"));
        };

        Ok(SwapResult {
            amount_out,
            gas_used,
            price_impact_bps: 0,
            logs: Vec::new(),
        })
    }

    fn get_quote(
        _token_in: H256,
        _token_out: H256,
        amount_in: u128,
        _pool_address: &[u8],
    ) -> Result<u128, DispatchError> {
        let fee_adjusted = amount_in.saturating_mul(997) / 1000;
        Ok(fee_adjusted)
    }

    fn calculate_price_impact(
        amount_in: u128,
        amount_out: u128,
        reserve_in: u128,
        reserve_out: u128,
    ) -> u32 {
        UniswapV2Adapter::calculate_price_impact(amount_in, amount_out, reserve_in, reserve_out)
    }
}

// ============================================================================
// Raydium Adapter (SVM)
// ============================================================================

/// Raydium AMM adapter for Solana
pub struct RaydiumAdapter;

impl AmmAdapter for RaydiumAdapter {
    fn protocol() -> AmmProtocol {
        AmmProtocol::Raydium
    }

    fn vm_type() -> VmType {
        VmType::Svm
    }

    fn build_swap_calldata(params: &SwapParams) -> Result<Vec<u8>, DispatchError> {
        let mut instruction_data = Vec::new();

        // Raydium swap instruction discriminator
        instruction_data.extend_from_slice(&[0xf8, 0xc6, 0x9e, 0x91, 0xe1, 0x75, 0x87, 0xc8]);

        // amount_in (u64, little-endian)
        let amount_in_u64 = params.amount_in.min(u64::MAX as u128) as u64;
        instruction_data.extend_from_slice(&amount_in_u64.to_le_bytes());

        // minimum_amount_out (u64, little-endian)
        let min_out_u64 = params.min_amount_out.min(u64::MAX as u128) as u64;
        instruction_data.extend_from_slice(&min_out_u64.to_le_bytes());

        Ok(instruction_data)
    }

    fn parse_swap_result(return_data: &[u8], gas_used: u64) -> Result<SwapResult, DispatchError> {
        let amount_out = if return_data.len() >= 8 {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&return_data[..8]);
            u64::from_le_bytes(bytes) as u128
        } else {
            return Err(DispatchError::Other("Invalid return data"));
        };

        Ok(SwapResult {
            amount_out,
            gas_used,
            price_impact_bps: 0,
            logs: Vec::new(),
        })
    }

    fn get_quote(
        _token_in: H256,
        _token_out: H256,
        amount_in: u128,
        _pool_address: &[u8],
    ) -> Result<u128, DispatchError> {
        let fee_adjusted = amount_in.saturating_mul(9975) / 10000;
        Ok(fee_adjusted)
    }

    fn calculate_price_impact(
        amount_in: u128,
        amount_out: u128,
        reserve_in: u128,
        reserve_out: u128,
    ) -> u32 {
        UniswapV2Adapter::calculate_price_impact(amount_in, amount_out, reserve_in, reserve_out)
    }
}

// ============================================================================
// Orca Whirlpool Adapter (SVM)
// ============================================================================

/// Orca Whirlpool concentrated liquidity adapter
pub struct OrcaWhirlpoolAdapter;

impl AmmAdapter for OrcaWhirlpoolAdapter {
    fn protocol() -> AmmProtocol {
        AmmProtocol::OrcaWhirlpool
    }

    fn vm_type() -> VmType {
        VmType::Svm
    }

    fn build_swap_calldata(params: &SwapParams) -> Result<Vec<u8>, DispatchError> {
        let mut instruction_data = Vec::new();

        // Orca Whirlpool swap instruction discriminator
        instruction_data.extend_from_slice(&[0xf8, 0xc6, 0x9e, 0x91, 0x77, 0x89, 0x12, 0xab]);

        // amount (u64)
        let amount_u64 = params.amount_in.min(u64::MAX as u128) as u64;
        instruction_data.extend_from_slice(&amount_u64.to_le_bytes());

        // other_amount_threshold (u64)
        let threshold_u64 = params.min_amount_out.min(u64::MAX as u128) as u64;
        instruction_data.extend_from_slice(&threshold_u64.to_le_bytes());

        // sqrt_price_limit (u128) - 0 for no limit
        instruction_data.extend_from_slice(&0u128.to_le_bytes());

        // amount_specified_is_input (bool)
        instruction_data.push(1);

        // a_to_b (bool)
        let a_to_b = params.extra_data.first().copied().unwrap_or(1);
        instruction_data.push(a_to_b);

        Ok(instruction_data)
    }

    fn parse_swap_result(return_data: &[u8], gas_used: u64) -> Result<SwapResult, DispatchError> {
        let amount_out = if return_data.len() >= 8 {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&return_data[..8]);
            u64::from_le_bytes(bytes) as u128
        } else {
            return Err(DispatchError::Other("Invalid return data"));
        };

        Ok(SwapResult {
            amount_out,
            gas_used,
            price_impact_bps: 0,
            logs: Vec::new(),
        })
    }

    fn get_quote(
        _token_in: H256,
        _token_out: H256,
        amount_in: u128,
        _pool_address: &[u8],
    ) -> Result<u128, DispatchError> {
        let fee_adjusted = amount_in.saturating_mul(997) / 1000;
        Ok(fee_adjusted)
    }

    fn calculate_price_impact(
        amount_in: u128,
        amount_out: u128,
        reserve_in: u128,
        reserve_out: u128,
    ) -> u32 {
        UniswapV2Adapter::calculate_price_impact(amount_in, amount_out, reserve_in, reserve_out)
    }
}

// ============================================================================
// Atlas Native AMM Adapter
// ============================================================================

/// Atlas Sphere native cross-VM AMM adapter
pub struct AtlasAmmAdapter;

impl AmmAdapter for AtlasAmmAdapter {
    fn protocol() -> AmmProtocol {
        AmmProtocol::AtlasAmm
    }

    fn vm_type() -> VmType {
        VmType::CrossVm
    }

    fn build_swap_calldata(params: &SwapParams) -> Result<Vec<u8>, DispatchError> {
        let mut calldata = Vec::new();

        // Operation code: 0x01 = swap
        calldata.push(0x01);

        // token_in
        calldata.extend_from_slice(params.token_in.as_bytes());

        // token_out
        calldata.extend_from_slice(params.token_out.as_bytes());

        // amount_in (u128, big-endian)
        calldata.extend_from_slice(&params.amount_in.to_be_bytes());

        // min_amount_out (u128, big-endian)
        calldata.extend_from_slice(&params.min_amount_out.to_be_bytes());

        // recipient length and data
        calldata.push(params.recipient.len() as u8);
        calldata.extend_from_slice(&params.recipient);

        Ok(calldata)
    }

    fn parse_swap_result(return_data: &[u8], gas_used: u64) -> Result<SwapResult, DispatchError> {
        if return_data.len() < 21 {
            return Err(DispatchError::Other("Invalid return data"));
        }

        let success = return_data[0] == 1;
        if !success {
            return Err(DispatchError::Other("Swap failed"));
        }

        let mut amount_bytes = [0u8; 16];
        amount_bytes.copy_from_slice(&return_data[1..17]);
        let amount_out = u128::from_be_bytes(amount_bytes);

        let mut impact_bytes = [0u8; 4];
        impact_bytes.copy_from_slice(&return_data[17..21]);
        let price_impact_bps = u32::from_be_bytes(impact_bytes);

        Ok(SwapResult {
            amount_out,
            gas_used,
            price_impact_bps,
            logs: Vec::new(),
        })
    }

    fn get_quote(
        _token_in: H256,
        _token_out: H256,
        amount_in: u128,
        _pool_address: &[u8],
    ) -> Result<u128, DispatchError> {
        let fee_adjusted = amount_in.saturating_mul(998) / 1000;
        Ok(fee_adjusted)
    }

    fn calculate_price_impact(
        amount_in: u128,
        amount_out: u128,
        reserve_in: u128,
        reserve_out: u128,
    ) -> u32 {
        UniswapV2Adapter::calculate_price_impact(amount_in, amount_out, reserve_in, reserve_out)
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Encode u128 as uint256 (32 bytes, big-endian, zero-padded)
fn encode_uint256(value: u128) -> [u8; 32] {
    let mut result = [0u8; 32];
    result[16..].copy_from_slice(&value.to_be_bytes());
    result
}

/// Decode uint256 to u128 (ignores upper 128 bits)
fn decode_uint256(data: &[u8]) -> Result<u128, DispatchError> {
    if data.len() < 32 {
        return Err(DispatchError::Other("Data too short for uint256"));
    }
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&data[16..32]);
    Ok(u128::from_be_bytes(bytes))
}

/// Get VM type for a protocol
pub fn get_vm_type_for_protocol(protocol: AmmProtocol) -> VmType {
    match protocol {
        AmmProtocol::UniswapV2
        | AmmProtocol::UniswapV3
        | AmmProtocol::StableSwap
        | AmmProtocol::ConstantProduct => VmType::Evm,
        AmmProtocol::Raydium | AmmProtocol::OrcaWhirlpool => VmType::Svm,
        AmmProtocol::AtlasAmm => VmType::CrossVm,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uniswap_v2_calldata() {
        let params = SwapParams {
            token_in: H256::from_low_u64_be(1),
            token_out: H256::from_low_u64_be(2),
            amount_in: 1_000_000_000_000_000_000u128,
            min_amount_out: 900_000_000_000_000_000u128,
            recipient: vec![0xab; 20],
            deadline: 1234567890,
            extra_data: vec![],
        };

        let calldata = UniswapV2Adapter::build_swap_calldata(&params).unwrap();
        assert_eq!(&calldata[..4], &[0x38, 0xed, 0x17, 0x39]);
        assert!(calldata.len() > 100);
    }

    #[test]
    fn test_raydium_calldata() {
        let params = SwapParams {
            token_in: H256::from_low_u64_be(1),
            token_out: H256::from_low_u64_be(2),
            amount_in: 1_000_000_000u128,
            min_amount_out: 900_000_000u128,
            recipient: vec![0xab; 32],
            deadline: 0,
            extra_data: vec![],
        };

        let calldata = RaydiumAdapter::build_swap_calldata(&params).unwrap();
        assert_eq!(
            &calldata[..8],
            &[0xf8, 0xc6, 0x9e, 0x91, 0xe1, 0x75, 0x87, 0xc8]
        );
    }

    #[test]
    fn test_price_impact_calculation() {
        let impact = UniswapV2Adapter::calculate_price_impact(
            1_000_000_000_000_000_000u128,
            1_960_000_000_000_000_000u128,
            100_000_000_000_000_000_000u128,
            200_000_000_000_000_000_000u128,
        );

        assert!(impact > 0);
        assert!(impact < 500);
    }

    #[test]
    fn test_encode_decode_uint256() {
        let original: u128 = 123_456_789_012_345_678_901_234u128;
        let encoded = encode_uint256(original);
        let decoded = decode_uint256(&encoded).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_vm_type_for_protocol() {
        assert_eq!(
            get_vm_type_for_protocol(AmmProtocol::UniswapV2),
            VmType::Evm
        );
        assert_eq!(get_vm_type_for_protocol(AmmProtocol::Raydium), VmType::Svm);
        assert_eq!(
            get_vm_type_for_protocol(AmmProtocol::AtlasAmm),
            VmType::CrossVm
        );
    }
}
