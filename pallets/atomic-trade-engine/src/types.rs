//! Extended types for the Atomic Trade Engine
//!
//! This module contains additional type definitions for trade execution,
//! path resolution, and AMM integration.

use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::{H256, U256};
use sp_runtime::RuntimeDebug;
use sp_std::vec::Vec;

// ============================================================================
// Core Types (exported to crate root)
// ============================================================================

/// Identifies the target VM for execution.
#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum VmType {
    /// Ethereum Virtual Machine
    Evm,
    /// Solana Virtual Machine
    Svm,
    /// Cross-VM operation (requires both)
    CrossVm,
}

/// AMM protocol identifiers.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Encode,
    Decode,
    RuntimeDebug,
    TypeInfo,
    MaxEncodedLen,
    Ord,
    PartialOrd,
)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum AmmProtocol {
    /// Uniswap V2 style AMM (EVM)
    UniswapV2,
    /// Uniswap V3 concentrated liquidity (EVM)
    UniswapV3,
    /// Raydium AMM (SVM)
    Raydium,
    /// Orca Whirlpool (SVM)
    OrcaWhirlpool,
    /// Custom Atlas Sphere AMM
    AtlasAmm,
    /// Generic constant product AMM
    ConstantProduct,
    /// Curve-style stable swap
    StableSwap,
}

// ============================================================================
// Trade Types
// ============================================================================

/// Represents a tradeable asset in the system.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct Asset {
    /// Unique asset identifier (H256 for cross-VM compatibility)
    pub id: H256,
    /// Human-readable symbol (e.g., "ETH", "SOL")
    pub symbol: Vec<u8>,
    /// Decimal precision
    pub decimals: u8,
    /// Native VM for this asset
    pub native_vm: VmType,
    /// Contract/program address on native VM
    pub address: Vec<u8>,
}

/// Liquidity pool information for trade routing.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct LiquidityPool {
    /// Pool identifier
    pub pool_id: H256,
    /// AMM protocol
    pub protocol: AmmProtocol,
    /// VM hosting this pool
    pub vm_type: VmType,
    /// Token A
    pub token_a: H256,
    /// Token B
    pub token_b: H256,
    /// Reserve of token A (scaled)
    pub reserve_a: u128,
    /// Reserve of token B (scaled)
    pub reserve_b: u128,
    /// Fee in basis points
    pub fee_bps: u32,
    /// Pool address/account
    pub address: Vec<u8>,
}

impl LiquidityPool {
    /// Calculate output amount for a given input using constant product formula.
    ///
    /// Formula: amount_out = (reserve_out * amount_in * (10000 - fee_bps)) / (reserve_in * 10000 + amount_in * (10000 - fee_bps))
    /// Uses U256 for intermediate calculations to avoid overflow with large token amounts.
    pub fn get_amount_out(&self, amount_in: u128, token_in: H256) -> Option<u128> {
        let (reserve_in, reserve_out) = if token_in == self.token_a {
            (self.reserve_a, self.reserve_b)
        } else if token_in == self.token_b {
            (self.reserve_b, self.reserve_a)
        } else {
            return None;
        };

        if reserve_in == 0 || reserve_out == 0 || amount_in == 0 {
            return None;
        }

        // Use U256 to avoid overflow in intermediate calculations
        let fee_multiplier = U256::from(10000u128.checked_sub(self.fee_bps as u128)?);
        let amount_in_with_fee = U256::from(amount_in) * fee_multiplier;
        let numerator = U256::from(reserve_out) * amount_in_with_fee;
        let denominator = U256::from(reserve_in) * U256::from(10000u64) + amount_in_with_fee;

        if denominator.is_zero() {
            return None;
        }

        let result = numerator / denominator;

        // Convert back to u128, returning None if overflow
        if result > U256::from(u128::MAX) {
            return None;
        }
        Some(result.as_u128())
    }

    /// Calculate required input amount for a desired output.
    ///
    /// Formula: amount_in = (reserve_in * amount_out * 10000) / ((reserve_out - amount_out) * (10000 - fee_bps))
    /// Uses U256 for intermediate calculations to avoid overflow with large token amounts.
    pub fn get_amount_in(&self, amount_out: u128, token_out: H256) -> Option<u128> {
        let (reserve_in, reserve_out) = if token_out == self.token_b {
            (self.reserve_a, self.reserve_b)
        } else if token_out == self.token_a {
            (self.reserve_b, self.reserve_a)
        } else {
            return None;
        };

        if reserve_in == 0 || reserve_out == 0 || amount_out == 0 || amount_out >= reserve_out {
            return None;
        }

        // Use U256 to avoid overflow in intermediate calculations
        let fee_multiplier = U256::from(10000u128.checked_sub(self.fee_bps as u128)?);
        let numerator = U256::from(reserve_in) * U256::from(amount_out) * U256::from(10000u64);
        let denominator = (U256::from(reserve_out) - U256::from(amount_out)) * fee_multiplier;

        if denominator.is_zero() {
            return None;
        }

        // Add 1 to round up
        let result = numerator / denominator + U256::one();

        // Convert back to u128, returning None if overflow
        if result > U256::from(u128::MAX) {
            return None;
        }
        Some(result.as_u128())
    }

    /// Calculate price impact for a trade.
    /// Uses U256 for intermediate calculations to avoid overflow with large token amounts.
    pub fn calculate_price_impact(&self, amount_in: u128, token_in: H256) -> Option<u32> {
        let amount_out = self.get_amount_out(amount_in, token_in)?;

        let (reserve_in, reserve_out) = if token_in == self.token_a {
            (self.reserve_a, self.reserve_b)
        } else {
            (self.reserve_b, self.reserve_a)
        };

        // Spot price = reserve_out / reserve_in
        // Execution price = amount_out / amount_in
        // Price impact = 1 - (execution_price / spot_price)

        // Use U256 for intermediate calculations
        let scale = U256::from(1_000_000u64);
        let spot_price_scaled = U256::from(reserve_out) * scale / U256::from(reserve_in);

        if spot_price_scaled.is_zero() {
            return Some(0);
        }

        let exec_price_scaled = U256::from(amount_out) * scale / U256::from(amount_in);

        if exec_price_scaled >= spot_price_scaled {
            return Some(0);
        }

        let impact_scaled = (spot_price_scaled - exec_price_scaled)
            * U256::from(10000u64) // Convert to basis points
            / spot_price_scaled;

        // Safe to cast as impact should be < 10000 basis points
        Some(impact_scaled.as_u32())
    }
}

/// Route step in a multi-hop trade path.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct RouteStep {
    /// Pool to use for this step
    pub pool_id: H256,
    /// Token being swapped in
    pub token_in: H256,
    /// Token being received
    pub token_out: H256,
    /// AMM protocol
    pub protocol: AmmProtocol,
    /// VM type
    pub vm_type: VmType,
}

/// Complete trade route with expected outputs.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct TradeRoute {
    /// Ordered steps in the route
    pub steps: Vec<RouteStep>,
    /// Starting token
    pub token_start: H256,
    /// Final token
    pub token_end: H256,
    /// Input amount
    pub amount_in: u128,
    /// Expected output amount (before slippage)
    pub expected_amount_out: u128,
    /// Total estimated gas
    pub estimated_gas: u64,
    /// Price impact in basis points
    pub price_impact_bps: u32,
}

impl TradeRoute {
    /// Check if route crosses VMs.
    pub fn is_cross_vm(&self) -> bool {
        if self.steps.is_empty() {
            return false;
        }
        let first_vm = self.steps[0].vm_type;
        self.steps.iter().any(|s| s.vm_type != first_vm)
    }

    /// Get total number of hops.
    pub fn hop_count(&self) -> usize {
        self.steps.len()
    }

    /// Calculate minimum output with slippage.
    pub fn min_amount_out(&self, slippage_bps: u32) -> u128 {
        let slippage_factor = 10000u128.saturating_sub(slippage_bps as u128);
        self.expected_amount_out.saturating_mul(slippage_factor) / 10000
    }
}

/// Quote response for trade simulation.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct TradeQuote {
    /// Best route found
    pub route: TradeRoute,
    /// Alternative routes (if any)
    pub alternatives: Vec<TradeRoute>,
    /// Quote validity (block number)
    pub valid_until: u64,
    /// Whether arbitrage opportunity exists
    pub arbitrage_available: bool,
    /// Expected arbitrage profit in basis points (if applicable)
    pub arbitrage_profit_bps: Option<u32>,
}

/// Single price observation for oracle storage.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, Default)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct PriceObservation {
    /// Observed price (scaled by 1e18)
    pub price: u128,
    /// Timestamp of observation
    pub timestamp: u64,
    /// Block number when recorded
    pub block_number: u64,
    /// Source identifier (AMM pool or oracle)
    pub source: u8,
}

/// Price oracle data point.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct PricePoint {
    /// Asset pair
    pub token_a: H256,
    pub token_b: H256,
    /// Price (token_b per token_a, scaled by 1e18)
    pub price: u128,
    /// Timestamp
    pub timestamp: u64,
    /// Block number
    pub block_number: u64,
    /// Source AMM
    pub source: AmmProtocol,
}

/// Time-weighted average price data.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct TwapData {
    /// Asset pair
    pub token_a: H256,
    pub token_b: H256,
    /// Cumulative price (for TWAP calculation)
    pub cumulative_price: u128,
    /// Last update timestamp
    pub last_timestamp: u64,
    /// Number of observations
    pub observation_count: u32,
    /// Window size in seconds
    pub window_seconds: u64,
}

impl TwapData {
    /// Calculate TWAP from cumulative price.
    pub fn calculate_twap(&self, current_cumulative: u128, current_timestamp: u64) -> Option<u128> {
        let time_elapsed = current_timestamp.checked_sub(self.last_timestamp)?;
        if time_elapsed == 0 {
            return None;
        }

        let price_diff = current_cumulative.checked_sub(self.cumulative_price)?;
        price_diff.checked_div(time_elapsed as u128)
    }
}

/// Arbitrage opportunity detection result.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct ArbitrageOpportunity {
    /// Circular path for arbitrage
    pub path: Vec<RouteStep>,
    /// Starting/ending token
    pub base_token: H256,
    /// Optimal input amount
    pub optimal_input: u128,
    /// Expected output (should be > input for profit)
    pub expected_output: u128,
    /// Profit in basis points
    pub profit_bps: u32,
    /// Gas cost estimate
    pub gas_estimate: u64,
    /// Net profitable after gas
    pub net_profitable: bool,
}

impl ArbitrageOpportunity {
    /// Calculate net profit after gas costs.
    pub fn net_profit(&self, gas_price: u128) -> i128 {
        let gross_profit = self.expected_output as i128 - self.optimal_input as i128;
        let gas_cost = (self.gas_estimate as u128).saturating_mul(gas_price) as i128;
        gross_profit - gas_cost
    }
}

/// Order type for limit orders.
#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum OrderType {
    /// Market order - execute immediately at current price
    Market,
    /// Limit order - execute only at specified price or better
    Limit,
    /// Stop-loss order - execute when price falls below threshold
    StopLoss,
    /// Take-profit order - execute when price rises above threshold
    TakeProfit,
}

/// Order side.
#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum OrderSide {
    Buy,
    Sell,
}

/// Pending limit order.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct LimitOrder<AccountId> {
    /// Order ID
    pub order_id: H256,
    /// Owner account
    pub owner: AccountId,
    /// Order type
    pub order_type: OrderType,
    /// Buy or sell
    pub side: OrderSide,
    /// Token to trade from
    pub token_in: H256,
    /// Token to receive
    pub token_out: H256,
    /// Amount of token_in
    pub amount_in: u128,
    /// Trigger price (scaled by 1e18)
    pub trigger_price: u128,
    /// Minimum amount out (slippage protection)
    pub min_amount_out: u128,
    /// Created at block
    pub created_at: u64,
    /// Expires at block
    pub expires_at: u64,
    /// Preferred route (optional)
    pub preferred_route: Option<Vec<RouteStep>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_pool() -> LiquidityPool {
        LiquidityPool {
            pool_id: H256::from_low_u64_be(1),
            protocol: AmmProtocol::UniswapV2,
            vm_type: VmType::Evm,
            token_a: H256::from_low_u64_be(100),
            token_b: H256::from_low_u64_be(200),
            reserve_a: 1_000_000_000_000_000_000u128, // 1e18
            reserve_b: 2_000_000_000_000_000_000u128, // 2e18
            fee_bps: 30,                              // 0.3%
            address: vec![0u8; 20],
        }
    }

    #[test]
    fn test_get_amount_out() {
        let pool = create_test_pool();
        let amount_in = 1_000_000_000_000_000u128; // 0.001 token_a

        let amount_out = pool.get_amount_out(amount_in, pool.token_a).unwrap();

        // With reserves 1:2 and 0.3% fee, ~0.002 token_b expected minus slippage
        assert!(amount_out > 0);
        assert!(amount_out < 2_000_000_000_000_000u128); // Less than 2x due to fee
    }

    #[test]
    fn test_get_amount_out_zero_input() {
        let pool = create_test_pool();
        assert!(pool.get_amount_out(0, pool.token_a).is_none());
    }

    #[test]
    fn test_get_amount_out_wrong_token() {
        let pool = create_test_pool();
        let wrong_token = H256::from_low_u64_be(999);
        assert!(pool.get_amount_out(1000, wrong_token).is_none());
    }

    #[test]
    fn test_get_amount_in() {
        let pool = create_test_pool();
        let desired_out = 1_000_000_000_000_000u128; // 0.001 token_b

        let amount_in = pool.get_amount_in(desired_out, pool.token_b).unwrap();

        // Should require some token_a to get this output
        assert!(amount_in > 0);
    }

    #[test]
    fn test_price_impact() {
        let pool = create_test_pool();

        // Small trade should have minimal impact
        let small_amount = 1_000_000_000_000u128; // 0.000001 token
        let small_impact = pool
            .calculate_price_impact(small_amount, pool.token_a)
            .unwrap();
        assert!(small_impact < 100); // Less than 1%

        // Large trade should have significant impact
        let large_amount = 100_000_000_000_000_000u128; // 0.1 token (10% of reserves)
        let large_impact = pool
            .calculate_price_impact(large_amount, pool.token_a)
            .unwrap();
        assert!(large_impact > small_impact);
    }

    #[test]
    fn test_trade_route_cross_vm() {
        let evm_step = RouteStep {
            pool_id: H256::from_low_u64_be(1),
            token_in: H256::from_low_u64_be(100),
            token_out: H256::from_low_u64_be(200),
            protocol: AmmProtocol::UniswapV2,
            vm_type: VmType::Evm,
        };

        let svm_step = RouteStep {
            pool_id: H256::from_low_u64_be(2),
            token_in: H256::from_low_u64_be(200),
            token_out: H256::from_low_u64_be(300),
            protocol: AmmProtocol::Raydium,
            vm_type: VmType::Svm,
        };

        let evm_only_route = TradeRoute {
            steps: vec![evm_step.clone()],
            token_start: H256::from_low_u64_be(100),
            token_end: H256::from_low_u64_be(200),
            amount_in: 1000,
            expected_amount_out: 2000,
            estimated_gas: 100000,
            price_impact_bps: 10,
        };

        let cross_vm_route = TradeRoute {
            steps: vec![evm_step, svm_step],
            token_start: H256::from_low_u64_be(100),
            token_end: H256::from_low_u64_be(300),
            amount_in: 1000,
            expected_amount_out: 3000,
            estimated_gas: 200000,
            price_impact_bps: 20,
        };

        assert!(!evm_only_route.is_cross_vm());
        assert!(cross_vm_route.is_cross_vm());
    }

    #[test]
    fn test_min_amount_out_with_slippage() {
        let route = TradeRoute {
            steps: vec![],
            token_start: H256::from_low_u64_be(100),
            token_end: H256::from_low_u64_be(200),
            amount_in: 1000,
            expected_amount_out: 10000,
            estimated_gas: 100000,
            price_impact_bps: 10,
        };

        // 1% slippage = 100 bps
        let min_out = route.min_amount_out(100);
        assert_eq!(min_out, 9900); // 10000 * 0.99

        // 5% slippage = 500 bps
        let min_out_5pct = route.min_amount_out(500);
        assert_eq!(min_out_5pct, 9500); // 10000 * 0.95
    }

    #[test]
    fn test_twap_calculation() {
        let twap = TwapData {
            token_a: H256::from_low_u64_be(100),
            token_b: H256::from_low_u64_be(200),
            cumulative_price: 1_000_000_000_000u128,
            last_timestamp: 1000,
            observation_count: 10,
            window_seconds: 3600,
        };

        // 100 seconds later, cumulative increased by 200_000_000_000
        let current_cumulative = 1_200_000_000_000u128;
        let current_timestamp = 1100u64;

        let avg_price = twap
            .calculate_twap(current_cumulative, current_timestamp)
            .unwrap();
        assert_eq!(avg_price, 2_000_000_000u128); // 200B / 100s = 2B per second
    }

    #[test]
    fn test_arbitrage_net_profit() {
        let arb = ArbitrageOpportunity {
            path: vec![],
            base_token: H256::from_low_u64_be(100),
            optimal_input: 1_000_000_000_000_000_000u128, // 1e18
            expected_output: 1_010_000_000_000_000_000u128, // 1.01e18 (1% profit)
            profit_bps: 100,
            gas_estimate: 500_000,
            net_profitable: true,
        };

        // Gas price: 10 gwei = 10_000_000_000
        let gas_price = 10_000_000_000u128;
        let net = arb.net_profit(gas_price);

        // Gross profit: 0.01e18 = 10_000_000_000_000_000
        // Gas cost: 500_000 * 10_000_000_000 = 5_000_000_000_000_000
        // Net: 10_000_000_000_000_000 - 5_000_000_000_000_000 = 5_000_000_000_000_000
        assert_eq!(net, 5_000_000_000_000_000i128);
    }
}
