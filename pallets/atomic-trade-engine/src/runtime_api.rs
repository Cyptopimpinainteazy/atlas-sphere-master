//! Runtime API definitions for the Atomic Trade Engine pallet.
//!
//! These APIs expose pallet functionality to RPC clients and external tools,
//! enabling AI agents and frontends to query trade simulation, pricing, and
//! execution cost estimation without submitting transactions.

use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_core::H256;
use sp_runtime::RuntimeDebug;
use sp_std::vec::Vec;

use crate::types::{RouteStep, TradeRoute};

/// Trade simulation result returned by the runtime API.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, Default)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct SimulationResult {
    /// Whether the trade would succeed
    pub success: bool,
    /// Estimated output amount
    pub estimated_output: u128,
    /// Price impact in basis points
    pub price_impact_bps: u32,
    /// EVM gas estimate
    pub evm_gas: u64,
    /// SVM compute units estimate
    pub svm_compute: u64,
    /// Execution path taken
    pub route: Vec<RouteStep>,
    /// Error message if simulation failed
    pub error: Option<Vec<u8>>,
}

/// Batch status for query.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, Default)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct BatchStatusResponse {
    /// Batch exists
    pub exists: bool,
    /// Current status
    pub status: u8, // 0=pending, 1=executing, 2=finalized, 3=rolled_back
    /// Submission block
    pub submitted_at: u64,
    /// Finalization block (if finalized)
    pub finalized_at: Option<u64>,
    /// Execution receipts count
    pub legs_executed: u32,
    /// Checkpoints saved
    pub checkpoints: u32,
}

/// Price oracle data response.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, Default)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct PriceDataResponse {
    /// Token pair exists in oracle
    pub exists: bool,
    /// Current TWAP price
    pub twap_price: Option<u128>,
    /// Latest spot price
    pub latest_price: Option<u128>,
    /// Number of observations
    pub observation_count: u32,
    /// Last update timestamp
    pub last_updated: u64,
}

sp_api::decl_runtime_apis! {
    /// Runtime API for the Atomic Trade Engine pallet.
    ///
    /// Provides query methods for AI agents, frontends, and external services
    /// to interact with the atomic trade engine without submitting transactions.
    pub trait AtomicTradeEngineApi {
        /// Simulate a trade path without executing it.
        ///
        /// # Arguments
        /// * `token_in` - Input token identifier
        /// * `token_out` - Output token identifier
        /// * `amount_in` - Amount of input token
        /// * `slippage_bps` - Maximum allowed slippage in basis points
        ///
        /// # Returns
        /// Simulation result with estimated output, gas costs, and route taken.
        fn simulate_trade(
            token_in: H256,
            token_out: H256,
            amount_in: u128,
            slippage_bps: u32,
        ) -> SimulationResult;

        /// Estimate execution costs for a multi-leg trade.
        ///
        /// # Arguments
        /// * `legs` - Number of trade legs
        /// * `vm_types` - VM type for each leg (0=EVM, 1=SVM, 2=CrossVM)
        ///
        /// # Returns
        /// Tuple of (evm_gas, svm_compute_units)
        fn estimate_execution_cost(
            legs: u32,
            vm_types: Vec<u8>,
        ) -> (u64, u64);

        /// Get the current TWAP price for a token pair.
        ///
        /// # Arguments
        /// * `token_a` - First token identifier
        /// * `token_b` - Second token identifier
        ///
        /// # Returns
        /// Price data including TWAP, latest price, and observation metadata.
        fn get_price_data(
            token_a: H256,
            token_b: H256,
        ) -> PriceDataResponse;

        /// Get batch status by hash.
        ///
        /// # Arguments
        /// * `batch_hash` - Hash of the trade batch
        ///
        /// # Returns
        /// Batch status including execution state and checkpoint info.
        fn get_batch_status(batch_hash: H256) -> BatchStatusResponse;

        /// Find optimal route between two tokens.
        ///
        /// # Arguments
        /// * `token_in` - Input token identifier
        /// * `token_out` - Output token identifier
        /// * `amount_in` - Amount to trade
        ///
        /// # Returns
        /// Optional route with expected output, or None if no path exists.
        fn find_route(
            token_in: H256,
            token_out: H256,
            amount_in: u128,
        ) -> Option<TradeRoute>;

        /// Check if an account is authorized for atomic trades.
        ///
        /// # Arguments
        /// * `account` - Account ID as bytes
        ///
        /// # Returns
        /// True if authorized.
        fn is_authorized(account: Vec<u8>) -> bool;
    }
}
