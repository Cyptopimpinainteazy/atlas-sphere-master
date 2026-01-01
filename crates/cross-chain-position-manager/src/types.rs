//! Core type definitions for Cross-Chain Position Manager
//!
//! This module defines all the fundamental types used throughout the system,
//! ensuring consistency and type safety across different modules.

use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_core::{H160, H256, U256};
use sp_std::vec::Vec;
use serde::{Deserialize, Serialize};

/// Unique identifier for a cross-chain position
#[derive(Debug, Clone, PartialEq, Eq, Hash, Encode, Decode, TypeInfo)]
pub struct PositionId(pub [u8; 32]);

impl PositionId {
    /// Create a new random position ID
    pub fn new() -> Self {
        let mut bytes = [0u8; 32];
        getrandom::getrandom(&mut bytes).expect("Failed to generate random bytes");
        Self(bytes)
    }

    /// Create from existing bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Get the underlying bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        format!("0x{:x}", H256::from(self.0))
    }
}

impl Default for PositionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for PositionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// Type of position held
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, TypeInfo)]
pub enum PositionType {
    /// Simple token balance
    Token,
    /// Liquidity provider position
    LpPosition,
    /// Lending position (supplying assets)
    LendingSupply,
    /// Borrowing position (borrowing assets)
    LendingBorrow,
    /// Staked assets
    Staked,
    /// Derivative position
    Derivative,
    /// Strategy position (complex)
    Strategy,
    /// Multi-asset portfolio
    Portfolio,
}

impl PositionType {
    /// Get risk level associated with this position type
    pub fn risk_level(&self) -> RiskLevel {
        match self {
            PositionType::Token => RiskLevel::Low,
            PositionType::LpPosition => RiskLevel::Medium,
            PositionType::LendingSupply => RiskLevel::Low,
            PositionType::LendingBorrow => RiskLevel::High,
            PositionType::Staked => RiskLevel::Medium,
            PositionType::Derivative => RiskLevel::High,
            PositionType::Strategy => RiskLevel::High,
            PositionType::Portfolio => RiskLevel::Medium,
        }
    }

    /// Get estimated gas cost for position operations
    pub fn gas_cost_estimate(&self) -> U256 {
        match self {
            PositionType::Token => U256::from(50_000),
            PositionType::LpPosition => U256::from(300_000),
            PositionType::LendingSupply => U256::from(200_000),
            PositionType::LendingBorrow => U256::from(250_000),
            PositionType::Staked => U256::from(150_000),
            PositionType::Derivative => U256::from(500_000),
            PositionType::Strategy => U256::from(1_000_000),
            PositionType::Portfolio => U256::from(800_000),
        }
    }
}

/// Risk level classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, TypeInfo)]
pub enum RiskLevel {
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

impl RiskLevel {
    /// Get numeric value
    pub fn value(&self) -> u8 {
        *self as u8
    }

    /// Check if this risk level is acceptable
    pub fn is_acceptable(&self, threshold: RiskLevel) -> bool {
        self.value() <= threshold.value()
    }
}

/// Current state of a position
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, TypeInfo)]
pub enum PositionState {
    /// Position is active and healthy
    Active,
    /// Position is being migrated
    Migrating,
    /// Position is being unwound
    Unwinding,
    /// Position is paused due to risk
    Paused,
    /// Position has failed
    Failed,
    /// Position is closed
    Closed,
}

/// Chain-specific configuration
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct ChainSpecifics {
    pub chain_id: u64,
    pub gas_price_multiplier: f64,
    pub min_gas_price: U256,
    pub max_gas_price: U256,
    pub bridge_timeout_ms: u64,
    pub confirmations_required: u64,
    pub native_token_decimals: u8,
    pub supports_eip1559: bool,
}

/// Asset information
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct AssetInfo {
    pub address: H160,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub is_native: bool,
    pub is_stable: bool,
    pub price_source: PriceSource,
    pub coingecko_id: Option<String>,
}

/// Source for price information
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub enum PriceSource {
    /// Price from a specific oracle
    Oracle(H160),
    /// Price from DEX pool
    DexPool { pool_address: H160, token0: H160, token1: H160 },
    /// Price from external API
    ExternalApi(String),
    /// Static price (for stablecoins)
    Static(U256),
    /// No price available
    None,
}

/// Position metadata
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct PositionMetadata {
    pub id: PositionId,
    pub position_type: PositionType,
    pub chain_id: u64,
    pub asset: AssetInfo,
    pub created_at: u64,
    pub last_updated: u64,
    pub tags: Vec<String>,
    pub strategy_id: Option<H256>,
}

/// Portfolio allocation target
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct AllocationTarget {
    pub chain_id: u64,
    pub asset: H160,
    pub target_percentage: f64, // 0.0 to 1.0
    pub min_amount: U256,
    pub max_amount: U256,
}

/// Risk threshold configuration
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct RiskThreshold {
    pub max_position_size_usd: U256,
    pub max_exposure_per_chain: f64, // percentage
    pub max_correlation: f64,
    pub liquidation_threshold: f64,
    pub stop_loss_percentage: f64,
}

/// Performance metrics
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct PerformanceMetrics {
    pub total_return_usd: U256,
    pub daily_return: f64,
    pub weekly_return: f64,
    pub monthly_return: f64,
    pub sharpe_ratio: f64,
    pub max_drawdown: f64,
    pub volatility: f64,
    pub win_rate: f64,
}

/// Trade execution parameters
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct ExecutionParams {
    pub slippage_tolerance: f64, // 0.001 = 0.1%
    pub gas_price_multiplier: f64,
    pub deadline_seconds: u64,
    pub min_profit_usd: U256,
    pub max_slippage: f64,
}

/// Cross-chain operation status
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, TypeInfo)]
pub enum OperationStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

/// Migration plan for moving positions
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct MigrationPlan {
    pub position_id: PositionId,
    pub from_chain: u64,
    pub to_chain: u64,
    pub assets: Vec<(H160, U256)>,
    pub route: Vec<u64>, // intermediate chains
    pub estimated_gas: U256,
    pub estimated_time: u64,
    pub cost_usd: U256,
}

/// Rebalancing action
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct RebalanceAction {
    pub action_type: RebalanceActionType,
    pub chain_id: u64,
    pub asset: H160,
    pub amount: U256,
    pub expected_output: U256,
    pub gas_estimate: U256,
    pub priority: u8,
}

/// Types of rebalancing actions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, TypeInfo)]
pub enum RebalanceActionType {
    Buy,
    Sell,
    BridgeIn,
    BridgeOut,
    Stake,
    Unstake,
    LpAdd,
    LpRemove,
}

/// Arbitrage opportunity
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct ArbitrageOpportunity {
    pub id: H256,
    pub profit_usd: U256,
    pub confidence: f64, // 0.0 to 1.0
    pub routes: Vec<SwapRoute>,
    pub deadline: u64,
    pub min_capital: U256,
    pub max_capital: U256,
}

/// Route optimization parameters
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct RouteOptimizationParams {
    pub max_hops: u8,
    pub preferred_chains: Vec<u64>,
    pub avoid_chains: Vec<u64>,
    pub min_liquidity: U256,
    pub gas_weight: f64,
    pub time_weight: f64,
    pub slippage_weight: f64,
}

/// Risk assessment result
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct RiskAssessment {
    pub position_id: PositionId,
    pub overall_risk: RiskLevel,
    pub risk_factors: Vec<RiskFactor>,
    pub recommendations: Vec<String>,
    pub score: f64, // 0.0 to 1.0
}

/// Individual risk factor
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct RiskFactor {
    pub factor_type: RiskFactorType,
    pub severity: RiskLevel,
    pub description: String,
    pub mitigation: Option<String>,
}

/// Types of risk factors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, TypeInfo)]
pub enum RiskFactorType {
    Liquidity,
    Market,
    Technical,
    Operational,
    Regulatory,
    Counterparty,
    SmartContract,
}

/// Kill switch configuration
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct KillSwitchConfig {
    pub enabled: bool,
    pub trigger_conditions: Vec<TriggerCondition>,
    pub auto_actions: Vec<AutoAction>,
    pub notification_channels: Vec<String>,
}

/// Kill switch trigger condition
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct TriggerCondition {
    pub condition_type: TriggerConditionType,
    pub threshold_value: f64,
    pub chain_ids: Vec<u64>,
    pub position_types: Vec<PositionType>,
}

/// Types of trigger conditions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, TypeInfo)]
pub enum TriggerConditionType {
    PriceDrop,
    VolumeSpike,
    GasSpike,
    ChainLatency,
    LiquidityDrop,
    ErrorRate,
    UnauthorizedAccess,
}

/// Event types for the event system
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub enum EventType {
    PositionCreated,
    PositionUpdated,
    PositionMigrated,
    PositionClosed,
    RebalanceTriggered,
    ArbitrageExecuted,
    RiskAlert,
    KillSwitchTriggered,
    ChainDisconnected,
    MigrationCompleted,
}

/// Cross-chain message
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct CrossChainMessage {
    pub id: H256,
    pub source_chain: u64,
    pub dest_chain: u64,
    pub message_type: MessageType,
    pub payload: Vec<u8>,
    pub nonce: u64,
    pub timestamp: u64,
}

/// Types of cross-chain messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, TypeInfo)]
pub enum MessageType {
    PositionUpdate,
    MigrationRequest,
    MigrationComplete,
    RiskAlert,
    PriceUpdate,
    StateSync,
}

/// State synchronization data
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct StateSync {
    pub snapshot_hash: H256,
    pub positions: Vec<PositionId>,
    pub balances: Vec<(H160, U256)>,
    pub metadata: Vec<u8>,
}

/// Performance benchmarking
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct PerformanceBenchmark {
    pub operation: String,
    pub duration_ms: u64,
    pub gas_used: U256,
    pub success: bool,
    pub chain_id: u64,
    pub timestamp: u64,
}

/// Configuration for automated operations
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct AutomationConfig {
    pub auto_rebalance: bool,
    pub auto_arbitrage: bool,
    pub auto_hedge: bool,
    pub rebalance_threshold: f64,
    pub arbitrage_min_profit: U256,
    pub max_positions_per_chain: usize,
}

/// Health check status
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, TypeInfo)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
    Offline,
}

/// Health check result
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct HealthCheck {
    pub component: String,
    pub status: HealthStatus,
    pub last_check: u64,
    pub details: String,
    pub uptime_percentage: f64,
}

impl Default for HealthStatus {
    fn default() -> Self {
        HealthStatus::Healthy
    }
}
