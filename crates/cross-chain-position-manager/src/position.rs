//! Core position management structures
//! 
//! This module defines the fundamental position structures and types
//! used across the cross-chain position manager.

use crate::types::*;
use crate::error::{Result, PositionManagerError};
use sp_core::{H160, H256, U256};
use sp_std::vec::Vec;

/// Complete cross-chain position with all metadata and current state
#[derive(Debug, Clone)]
pub struct CrossChainPosition {
    pub id: PositionId,
    pub metadata: PositionMetadata,
    pub state: PositionState,
    pub chain_holdings: Vec<ChainHolding>,
    pub performance: PerformanceMetrics,
    pub risk_data: RiskAssessment,
    pub last_updated: u64,
    pub tags: Vec<String>,
}

/// Asset holdings on a specific chain
#[derive(Debug, Clone)]
pub struct ChainHolding {
    pub chain_id: u64,
    pub asset: AssetInfo,
    pub balance: U256,
    pub balance_usd: U256,
    pub contract_address: Option<H160>,
    pub additional_data: PositionAdditionalData,
}

/// Additional data specific to position type
#[derive(Debug, Clone)]
pub enum PositionAdditionalData {
    LpPosition(LpData),
    LendingData(LendingData),
    StakingData(StakingData),
    DerivativeData(DerivativeData),
    StrategyData(StrategyData),
    Token,
}

/// LP position specific data
#[derive(Debug, Clone)]
pub struct LpData {
    pub pool_address: H160,
    pub token_a: H160,
    pub token_b: H160,
    pub lp_tokens: U256,
    pub reserved_token_a: U256,
    pub reserved_token_b: U256,
    pub fees_earned_a: U256,
    pub fees_earned_b: U256,
    pub impermanent_loss: f64,
}

/// Lending position specific data
#[derive(Debug, Clone)]
pub struct LendingData {
    pub protocol: String,
    pub supplied_amount: U256,
    pub borrowed_amount: Option<U256>,
    pub interest_rate: f64,
    pub health_factor: Option<f64>,
    pub liquidation_threshold: f64,
}

/// Staking position specific data
#[derive(Debug, Clone)]
pub struct StakingData {
    pub staking_contract: H160,
    pub validator_address: Option<H160>,
    pub staked_amount: U256,
    pub rewards_earned: U256,
    pub unbonding_end: Option<u64>,
    pub lock_duration: Option<u64>,
}

/// Derivative position specific data
#[derive(Debug, Clone)]
pub struct DerivativeData {
    pub derivative_type: String,
    pub underlying: H160,
    pub size: U256,
    pub entry_price: U256,
    pub mark_price: U256,
    pub pnl: U256,
    pub margin_required: U256,
}

/// Strategy position specific data
#[derive(Debug, Clone)]
pub struct StrategyData {
    pub strategy_id: H256,
    pub strategy_name: String,
    pub underlying_positions: Vec<PositionId>,
    pub parameters: Vec<(String, String)>,
    pub expected_apy: f64,
    pub risk_level: RiskLevel,
}

impl CrossChainPosition {
    /// Create a new cross-chain position
    pub fn new(
        position_type: PositionType,
        chain_id: u64,
        asset: AssetInfo,
        initial_balance: U256,
    ) -> Self {
        let id = PositionId::new();
        let metadata = PositionMetadata {
            id: id.clone(),
            position_type,
            chain_id,
            asset: asset.clone(),
            created_at: current_timestamp(),
            last_updated: current_timestamp(),
            tags: Vec::new(),
            strategy_id: None,
        };

            chain_id,
            asset: asset.clone(),
            balance: initial_balance,
            balance_usd: U256::zero(),
            contract_address: None,
            additional_data: PositionAdditionalData::Token,
        }];

        Self {
            id,
            metadata,
            state: PositionState::Active,
            chain_holdings,
            performance: PerformanceMetrics::default(),
            risk_data: RiskAssessment {
                position_id: PositionId::new(),
                overall_risk: RiskLevel::Low,
                risk_factors: Vec::new(),
                recommendations: Vec::new(),
                score: 0.0,
            },
            last_updated: current_timestamp(),
            tags: Vec::new(),
        }
    }

    /// Get total value across all chains
    pub fn total_value_usd(&self) -> U256 {
        self.chain_holdings
            .iter()
            .map(|holding| holding.balance_usd)
            .fold(U256::zero(), |acc, val| acc + val)
    }

    /// Update position balance
    pub fn update_balance(&mut self, chain_id: u64, new_balance: U256) -> Result<()> {
        for holding in &mut self.chain_holdings {
            if holding.chain_id == chain_id {
                holding.balance = new_balance;
                self.last_updated = current_timestamp();
                return Ok(());
            }
        }
        Err(PositionManagerError::AssetNotFound {
            asset_address: H160::zero(),
            chain_id,
        })
    }

    /// Add a new chain holding
    pub fn add_chain_holding(&mut self, chain_id: u64, asset: AssetInfo, balance: U256) -> Result<()> {
        self.chain_holdings.push(ChainHolding {
            chain_id,
            asset,
            balance,
            balance_usd: U256::zero(),
            contract_address: None,
            additional_data: PositionAdditionalData::Token,
        });
        self.last_updated = current_timestamp();
        Ok(())
    }
}

/// Helper function to get current timestamp
fn current_timestamp() -> u64 {
    // In a real implementation, this would get the actual current timestamp
    1704067200 // Example timestamp
}

