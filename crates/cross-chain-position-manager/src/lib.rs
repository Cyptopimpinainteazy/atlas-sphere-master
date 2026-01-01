//! Universal Cross-Chain Position Manager for Atlas Sphere
//!
//! This crate provides comprehensive cross-chain position management capabilities:
//! - Track positions across 103+ EVM chains
//! - Atomic cross-chain position migration via Comit bundles
//! - Autonomous rebalancing and arbitrage execution
//! - Real-time risk management with kill switches
//! - Integration with Evolution Core and AI agents
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                Cross-Chain Position Manager                 │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Position Tracking  │  Migration Engine  │  Rebalancing     │
//! │  • Token Balances   │  • Comit Bundles   │  • Target Alloc  │
//! │  • LP Positions     │  • Route Finding   │  • Volatility    │
//! │  • Lending/Borrow   │  • Atomic Swaps    │  • APY Tracking  │
//! │  • Staked Assets    │  • Slippage Calc   │  • Gas Optim     │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Arbitrage Engine   │  Risk Management   │  Event System    │
//! │  • Price Monitoring │  • Kill Switches   │  • Cross-chain   │
//! │  • Opportunity Find │  • Rug Detection   │  • Real-time     │
//! │  • Atomic Execution │  • Emergency Univ  │  • Async Events  │
//! └─────────────────────────────────────────────────────────────┘
//!                                │
//!                ┌───────────────┼───────────────┐
//!                │               │               │
//!         ┌──────────┐  ┌─────────────┐  ┌─────────────┐
//!         │ External │  │   Evolution │  │   GPU Swarm │
//!         │  Chains  │  │    Core     │  │ AI Agents   │
//!         └──────────┘  └─────────────┘  └─────────────┘
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod accounting;
pub mod adapters;
pub mod arbitrage;
pub mod config;
pub mod error;
pub mod events;
pub mod migration;
pub mod position;
pub mod rebalancing;
pub mod risk;
pub mod router;
pub mod state;
pub mod tracking;
pub mod types;
pub mod utils;

// Core exports
pub use accounting::{AccountingEngine, PositionSnapshot, UsdNormalizer};
pub use adapters::{UniversalChainAdapter, ChainRegistryAdapter, CrossChainAdapter};
pub use arbitrage::{ArbitrageDetector, ArbitrageExecutor, Opportunity};
pub use config::{PositionManagerConfig, ChainConfig, RiskConfig};
pub use error::{PositionManagerError, Result};
pub use events::{EventBus, Event, PositionEvent, ChainEvent, RiskEvent};
pub use migration::{MigrationEngine, MigrationPlan, AtomicBundle};
pub use position::{CrossChainPosition, PositionId, PositionType, PositionState};
pub use rebalancing::{RebalancingEngine, RebalancePlan, AllocationTarget};
pub use risk::{RiskManager, RiskAssessment, KillSwitch};
pub use router::{RouteOptimizer, SwapRoute, ExecutionPlan};
pub use state::{PositionStateManager, StateSnapshot, PersistenceLayer};
pub use tracking::{PositionTracker, BalanceTracker, StrategyTracker};
pub use types::*;

// Re-export common types from external dependencies
pub use atlas_external_chains::{
    ChainType, ChainAdapter, AtomicSwapBundle, SwapRoute, QuoteResult,
};
pub use sp_core::{H160, H256, U256};
pub use sp_std::vec::Vec;

/// Main entry point for the Cross-Chain Position Manager
pub struct CrossChainPositionManager {
    config: PositionManagerConfig,
    accounting: AccountingEngine,
    position_tracker: PositionTracker,
    migration_engine: MigrationEngine,
    rebalancing_engine: RebalancingEngine,
    arbitrage_engine: ArbitrageExecutor,
    risk_manager: RiskManager,
    state_manager: PositionStateManager,
    event_bus: EventBus,
    chain_adapters: UniversalChainAdapter,
}

impl CrossChainPositionManager {
    /// Create a new position manager with default configuration
    pub fn new() -> Result<Self> {
        Self::new_with_config(PositionManagerConfig::default())
    }

    /// Create a position manager with custom configuration
    pub fn new_with_config(config: PositionManagerConfig) -> Result<Self> {
        let accounting = AccountingEngine::new()?;
        let position_tracker = PositionTracker::new(&config)?;
        let migration_engine = MigrationEngine::new(&config)?;
        let rebalancing_engine = RebalancingEngine::new(&config)?;
        let arbitrage_engine = ArbitrageExecutor::new(&config)?;
        let risk_manager = RiskManager::new(&config.risk_config)?;
        let state_manager = PositionStateManager::new(&config)?;
        let event_bus = EventBus::new();
        let chain_adapters = UniversalChainAdapter::new(&config)?;

        Ok(Self {
            config,
            accounting,
            position_tracker,
            migration_engine,
            rebalancing_engine,
            arbitrage_engine,
            risk_manager,
            state_manager,
            event_bus,
            chain_adapters,
        })
    }

    /// Start the position manager (initializes all subsystems)
    pub async fn start(&mut self) -> Result<()> {
        tracing::info!("Starting Cross-Chain Position Manager...");

        // Initialize chain connections
        self.chain_adapters.connect_all().await?;

        // Start position tracking
        self.position_tracker.start().await?;

        // Start arbitrage detection
        self.arbitrage_engine.start().await?;

        // Start risk monitoring
        self.risk_manager.start().await?;

        // Start state persistence
        self.state_manager.start().await?;

        tracing::info!("Cross-Chain Position Manager started successfully");
        Ok(())
    }

    /// Stop the position manager gracefully
    pub async fn stop(&mut self) -> Result<()> {
        tracing::info!("Stopping Cross-Chain Position Manager...");

        self.arbitrage_engine.stop().await?;
        self.risk_manager.stop().await?;
        self.state_manager.stop().await?;
        self.position_tracker.stop().await?;

        tracing::info!("Cross-Chain Position Manager stopped");
        Ok(())
    }

    /// Get current portfolio summary across all chains
    pub async fn get_portfolio_summary(&self) -> Result<PortfolioSummary> {
        self.accounting.get_portfolio_summary().await
    }

    /// Track positions across all connected chains
    pub async fn track_positions(&self) -> Result<Vec<CrossChainPosition>> {
        self.position_tracker.track_all_positions().await
    }

    /// Migrate a position from one chain to another
    pub async fn migrate_position(
        &self,
        from_chain: u64,
        to_chain: u64,
        position_id: &PositionId,
    ) -> Result<MigrationResult> {
        self.migration_engine
            .migrate_position(from_chain, to_chain, position_id)
            .await
    }

    /// Rebalance portfolio according to target allocations
    pub async fn rebalance(&self, targets: &[AllocationTarget]) -> Result<RebalanceResult> {
        self.rebalancing_engine.rebalance(targets).await
    }

    /// Unwind a position on a specific chain
    pub async fn unwind_position(
        &self,
        chain_id: u64,
        position_id: &PositionId,
    ) -> Result<UnwindResult> {
        self.migration_engine.unwind_position(chain_id, position_id).await
    }

    /// Simulate a cross-chain position move
    pub async fn simulate_cross_chain_move(
        &self,
        from_chain: u64,
        to_chain: u64,
        asset: H160,
        amount: U256,
    ) -> Result<SimulationResult> {
        self.migration_engine
            .simulate_move(from_chain, to_chain, asset, amount)
            .await
    }

    /// Evaluate arbitrage opportunities
    pub async fn evaluate_arbitrage(&self) -> Result<Vec<ArbitrageOpportunity>> {
        self.arbitrage_engine.find_opportunities().await
    }

    /// Execute an atomic bundle
    pub async fn execute_atomic_bundle(
        &self,
        bundle: &AtomicBundle,
    ) -> Result<ExecutionResult> {
        self.migration_engine.execute_bundle(bundle).await
    }

    /// Get risk assessment for a position
    pub async fn assess_position_risk(&self, position_id: &PositionId) -> Result<RiskAssessment> {
        self.risk_manager.assess_position(position_id).await
    }

    /// Check if a kill switch should be triggered
    pub async fn check_kill_switches(&self) -> Result<Vec<KillSwitchTrigger>> {
        self.risk_manager.check_all_kill_switches().await
    }

    /// Get configuration
    pub fn config(&self) -> &PositionManagerConfig {
        &self.config
    }

    /// Get event bus for subscribing to events
    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }
}

impl Default for CrossChainPositionManager {
    fn default() -> Self {
        Self::new().expect("Failed to create default Position Manager")
    }
}

/// Portfolio summary across all chains
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PortfolioSummary {
    pub total_value_usd: U256,
    pub chain_breakdown: Vec<ChainSummary>,
    pub asset_breakdown: Vec<AssetSummary>,
    pub risk_score: f64,
    pub rebalance_needed: bool,
    pub active_arbitrage_ops: usize,
}

/// Summary for a specific chain
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChainSummary {
    pub chain_id: u64,
    pub chain_type: ChainType,
    pub total_value_usd: U256,
    pub positions_count: usize,
    pub gas_efficiency_score: f64,
}

/// Summary for a specific asset
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AssetSummary {
    pub asset_address: H160,
    pub symbol: String,
    pub total_amount: U256,
    pub total_value_usd: U256,
    pub chains_distribution: Vec<(u64, U256)>,
}

/// Result of a migration operation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MigrationResult {
    pub success: bool,
    pub migration_id: H256,
    pub estimated_duration_ms: u64,
    pub gas_cost_estimate: U256,
    pub slippage_estimate: f64,
    pub route: SwapRoute,
}

/// Result of a rebalancing operation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RebalanceResult {
    pub success: bool,
    pub rebalance_id: H256,
    pub actions_executed: usize,
    pub total_cost_usd: U256,
    pub improvement_estimate: f64,
}

/// Result of an unwind operation
#[derive(Debug, Clone, serde::Serialize, serde:: Deserialize)]
pub struct UnwindResult {
    pub success: bool,
    pub unwind_id: H256,
    pub recovered_value_usd: U256,
    pub gas_cost_estimate: U256,
}

/// Arbitrage opportunity discovered
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArbitrageOpportunity {
    pub opportunity_id: H256,
    pub profit_estimate_usd: U256,
    pub route: SwapRoute,
    pub confidence_score: f64,
    pub time_window_ms: u64,
}

/// Result of bundle execution
#[derive(Debug, Clone, serde::Serialize, serde:: Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub execution_id: H256,
    pub gas_used: U256,
    pub actual_slippage: f64,
    pub final_state: PositionState,
}

/// Simulation result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SimulationResult {
    pub feasible: bool,
    pub estimated_cost: U256,
    pub estimated_duration: u64,
    pub risks: Vec<String>,
    pub alternatives: Vec<SwapRoute>,
}

/// Kill switch trigger event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KillSwitchTrigger {
    pub chain_id: u64,
    pub trigger_type: KillSwitchType,
    pub severity: RiskSeverity,
    pub description: String,
    pub auto_action: AutoAction,
}

/// Type of kill switch
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KillSwitchType {
    ChainFailure,
    RugDetection,
    LiquidityCrisis,
    GasSpike,
    StrategyFailure,
    RiskThreshold,
}

/// Risk severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Automatic action to take
#[derive(Debug, Clone)]
pub enum AutoAction {
    None,
    PauseTrading,
    UnwindPositions,
    ConsolidateToT1,
    EmergencyStop,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_manager_creation() {
        let manager = CrossChainPositionManager::new();
        assert!(manager.is_ok());
    }

    #[test]
    fn test_portfolio_summary_structure() {
        let summary = PortfolioSummary {
            total_value_usd: U256::from(1000000),
            chain_breakdown: Vec::new(),
            asset_breakdown: Vec::new(),
            risk_score: 0.5,
            rebalance_needed: false,
            active_arbitrage_ops: 0,
        };
        
        assert_eq!(summary.total_value_usd, U256::from(1000000));
        assert!(summary.risk_score >= 0.0 && summary.risk_score <= 1.0);
    }
}
