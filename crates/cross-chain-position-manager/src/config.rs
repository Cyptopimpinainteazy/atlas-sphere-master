//! Configuration for Cross-Chain Position Manager
//!
//! This module defines all configuration structures used throughout the system,
//! providing flexibility for different deployment scenarios while maintaining
//! sensible defaults.

use sp_core::{H160, U256};
use sp_std::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::types::{
    ChainSpecifics, AssetInfo, RiskThreshold, AutomationConfig,
    KillSwitchConfig, RouteOptimizationParams, ExecutionParams
};

/// Main configuration for the Cross-Chain Position Manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionManagerConfig {
    /// General settings
    pub general: GeneralConfig,
    /// Chain-specific configurations
    pub chains: Vec<ChainConfig>,
    /// Risk management configuration
    pub risk_config: RiskConfig,
    /// Position tracking configuration
    pub tracking: TrackingConfig,
    /// Migration engine configuration
    pub migration: MigrationConfig,
    /// Rebalancing engine configuration
    pub rebalancing: RebalancingConfig,
    /// Arbitrage engine configuration
    pub arbitrage: ArbitrageConfig,
    /// State management configuration
    pub state: StateConfig,
    /// Event system configuration
    pub events: EventConfig,
    /// Integration configuration
    pub integrations: IntegrationConfig,
}

impl Default for PositionManagerConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            chains: Self::default_chains(),
            risk_config: RiskConfig::default(),
            tracking: TrackingConfig::default(),
            migration: MigrationConfig::default(),
            rebalancing: RebalancingConfig::default(),
            arbitrage: ArbitrageConfig::default(),
            state: StateConfig::default(),
            events: EventConfig::default(),
            integrations: IntegrationConfig::default(),
        }
    }
}

impl PositionManagerConfig {
    /// Create default configuration for common chains
    fn default_chains() -> Vec<ChainConfig> {
        vec![
            // Tier 1 chains
            ChainConfig {
                chain_id: 1, // Ethereum
                enabled: true,
                priority: 1,
                chain_specifics: ChainSpecifics {
                    chain_id: 1,
                    gas_price_multiplier: 1.0,
                    min_gas_price: U256::from(20_000_000_000u64), // 20 gwei
                    max_gas_price: U256::from(500_000_000_000u64), // 500 gwei
                    bridge_timeout_ms: 300_000, // 5 minutes
                    confirmations_required: 12,
                    native_token_decimals: 18,
                    supports_eip1559: true,
                },
                assets: vec![
                    AssetConfig {
                        address: H160::zero(),
                        symbol: "ETH".to_string(),
                        decimals: 18,
                        enabled: true,
                        tracking_enabled: true,
                    },
                    AssetConfig {
                        address: H160::from_low_u64_be(0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48), // USDC
                        symbol: "USDC".to_string(),
                        decimals: 6,
                        enabled: true,
                        tracking_enabled: true,
                    },
                ],
            },
            ChainConfig {
                chain_id: 8453, // Base
                enabled: true,
                priority: 1,
                chain_specifics: ChainSpecifics {
                    chain_id: 8453,
                    gas_price_multiplier: 0.1,
                    min_gas_price: U256::from(1_000_000u64), // Very low on Base
                    max_gas_price: U256::from(100_000_000u64),
                    bridge_timeout_ms: 180_000, // 3 minutes
                    confirmations_required: 2,
                    native_token_decimals: 18,
                    supports_eip1559: true,
                },
                assets: vec![
                    AssetConfig {
                        address: H160::zero(),
                        symbol: "ETH".to_string(),
                        decimals: 18,
                        enabled: true,
                        tracking_enabled: true,
                    },
                    AssetConfig {
                        address: H160::from_low_u64_be(0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913), // USDC
                        symbol: "USDC".to_string(),
                        decimals: 6,
                        enabled: true,
                        tracking_enabled: true,
                    },
                ],
            },
            ChainConfig {
                chain_id: 42161, // Arbitrum
                enabled: true,
                priority: 1,
                chain_specifics: ChainSpecifics {
                    chain_id: 42161,
                    gas_price_multiplier: 0.05,
                    min_gas_price: U256::from(100_000u64),
                    max_gas_price: U256::from(10_000_000u64),
                    bridge_timeout_ms: 240_000, // 4 minutes
                    confirmations_required: 20,
                    native_token_decimals: 18,
                    supports_eip1559: true,
                },
                assets: vec![
                    AssetConfig {
                        address: H160::zero(),
                        symbol: "ETH".to_string(),
                        decimals: 18,
                        enabled: true,
                        tracking_enabled: true,
                    },
                    AssetConfig {
                        address: H160::from_low_u64_be(0xFF970A61A04b1cA14834A43f5dE4533eBDDB5CC8), // USDC
                        symbol: "USDC".to_string(),
                        decimals: 6,
                        enabled: true,
                        tracking_enabled: true,
                    },
                ],
            },
        ]
    }

    /// Get configuration for a specific chain
    pub fn get_chain_config(&self, chain_id: u64) -> Option<&ChainConfig> {
        self.chains.iter().find(|c| c.chain_id == chain_id)
    }

    /// Get enabled chains
    pub fn enabled_chains(&self) -> Vec<u64> {
        self.chains.iter()
            .filter(|c| c.enabled)
            .map(|c| c.chain_id)
            .collect()
    }

    /// Get priority chains (Tier 1)
    pub fn priority_chains(&self) -> Vec<u64> {
        self.chains.iter()
            .filter(|c| c.enabled && c.priority == 1)
            .map(|c| c.chain_id)
            .collect()
    }
}

/// General configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// Enable debug logging
    pub debug: bool,
    /// Maximum concurrent operations
    pub max_concurrent_ops: usize,
    /// Default operation timeout (seconds)
    pub operation_timeout_secs: u64,
    /// Enable metrics collection
    pub enable_metrics: bool,
    /// Data directory for persistence
    pub data_dir: String,
    /// Enable API server
    pub enable_api: bool,
    /// API server port
    pub api_port: u16,
    /// Maximum positions to track
    pub max_positions: usize,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            debug: false,
            max_concurrent_ops: 100,
            operation_timeout_secs: 300,
            enable_metrics: true,
            data_dir: "./data".to_string(),
            enable_api: false,
            api_port: 8080,
            max_positions: 10_000,
        }
    }
}

/// Chain-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    pub chain_id: u64,
    pub enabled: bool,
    pub priority: u8, // 1 = Tier 1, 2 = Tier 2, 3 = Tier 3
    pub chain_specifics: ChainSpecifics,
    pub assets: Vec<AssetConfig>,
}

/// Asset configuration for a chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetConfig {
    pub address: H160,
    pub symbol: String,
    pub decimals: u8,
    pub enabled: bool,
    pub tracking_enabled: bool,
}

/// Risk management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskConfig {
    /// Maximum position size in USD
    pub max_position_size_usd: U256,
    /// Maximum exposure per chain (percentage)
    pub max_exposure_per_chain: f64,
    /// Global maximum exposure (percentage)
    pub max_global_exposure: f64,
    /// Risk thresholds
    pub thresholds: RiskThresholdsConfig,
    /// Kill switch configuration
    pub kill_switches: KillSwitchConfig,
    /// Liquidation thresholds
    pub liquidation_thresholds: LiquidationConfig,
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            max_position_size_usd: U256::from(1_000_000_000_000u64), // $1M
            max_exposure_per_chain: 0.3, // 30%
            max_global_exposure: 0.8, // 80%
            thresholds: RiskThresholdsConfig::default(),
            kill_switches: KillSwitchConfig::default(),
            liquidation_thresholds: LiquidationConfig::default(),
        }
    }
}

/// Risk thresholds configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskThresholdsConfig {
    /// Volatility threshold for rebalancing
    pub volatility_threshold: f64,
    /// Liquidity threshold (minimum liquidity in USD)
    pub min_liquidity_usd: U256,
    /// Gas price spike threshold (multiplier)
    pub gas_spike_threshold: f64,
    /// Chain latency threshold (milliseconds)
    pub chain_latency_threshold_ms: u64,
}

impl Default for RiskThresholdsConfig {
    fn default() -> Self {
        Self {
            volatility_threshold: 0.05, // 5%
            min_liquidity_usd: U256::from(100_000_000_000u64), // $100M
            gas_spike_threshold: 5.0, // 5x normal
            chain_latency_threshold_ms: 10_000, // 10 seconds
        }
    }
}

/// Liquidation thresholds configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidationConfig {
    /// Health factor threshold for Aave-like protocols
    pub health_factor_threshold: f64,
    /// Collateral ratio threshold
    pub collateral_ratio_threshold: f64,
    /// Liquidation penalty threshold
    pub liquidation_penalty_threshold: f64,
}

impl Default for LiquidationConfig {
    fn default() -> Self {
        Self {
            health_factor_threshold: 1.5,
            collateral_ratio_threshold: 1.5,
            liquidation_penalty_threshold: 0.05, // 5%
        }
    }
}

/// Position tracking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingConfig {
    /// Tracking interval (seconds)
    pub tracking_interval_secs: u64,
    /// Enable real-time tracking
    pub real_time_tracking: bool,
    /// Balance tracking settings
    pub balance_tracking: BalanceTrackingConfig,
    /// Strategy tracking settings
    pub strategy_tracking: StrategyTrackingConfig,
}

impl Default for TrackingConfig {
    fn default() -> Self {
        Self {
            tracking_interval_secs: 30,
            real_time_tracking: true,
            balance_tracking: BalanceTrackingConfig::default(),
            strategy_tracking: StrategyTrackingConfig::default(),
        }
    }
}

/// Balance tracking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceTrackingConfig {
    /// Enable token balance tracking
    pub enabled: bool,
    /// Minimum balance threshold to track
    pub min_balance_usd: U256,
    /// Update frequency (seconds)
    pub update_frequency_secs: u64,
}

impl Default for BalanceTrackingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_balance_usd: U256::from(100_000_000u64), // $100
            update_frequency_secs: 60,
        }
    }
}

/// Strategy tracking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyTrackingConfig {
    /// Enable strategy position tracking
    pub enabled: bool,
    /// Minimum strategy value to track
    pub min_strategy_value_usd: U256,
    /// Performance tracking interval (seconds)
    pub performance_interval_secs: u64,
}

impl Default for StrategyTrackingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_strategy_value_usd: U256::from(1_000_000_000u64), // $1K
            performance_interval_secs: 300, // 5 minutes
        }
    }
}

/// Migration engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationConfig {
    /// Enable automatic migrations
    pub auto_migrate: bool,
    /// Migration timeout (seconds)
    pub migration_timeout_secs: u64,
    /// Maximum gas price for migrations
    pub max_gas_price: U256,
    /// Slippage tolerance for migrations
    pub slippage_tolerance: f64,
    /// Enable atomic migrations only
    pub atomic_only: bool,
}

impl Default for MigrationConfig {
    fn default() -> Self {
        Self {
            auto_migrate: false,
            migration_timeout_secs: 1800, // 30 minutes
            max_gas_price: U256::from(1_000_000_000_000u64), // 1000 gwei
            slippage_tolerance: 0.01, // 1%
            atomic_only: true,
        }
    }
}

/// Rebalancing engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebalancingConfig {
    /// Enable automatic rebalancing
    pub auto_rebalance: bool,
    /// Rebalancing threshold (deviation from target)
    pub rebalance_threshold: f64,
    /// Minimum rebalance value in USD
    pub min_rebalance_usd: U256,
    /// Rebalancing frequency limit (seconds)
    pub min_rebalance_interval_secs: u64,
    /// Enable batch rebalancing
    pub batch_rebalance: bool,
}

impl Default for RebalancingConfig {
    fn default() -> Self {
        Self {
            auto_rebalance: false,
            rebalance_threshold: 0.05, // 5% deviation
            min_rebalance_usd: U256::from(100_000_000u64), // $100
            min_rebalance_interval_secs: 3600, // 1 hour
            batch_rebalance: true,
        }
    }
}

/// Arbitrage engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageConfig {
    /// Enable arbitrage detection
    pub enabled: bool,
    /// Minimum profit threshold in USD
    pub min_profit_usd: U256,
    /// Maximum profit per trade
    pub max_profit_usd: U256,
    /// Confidence threshold for execution
    pub confidence_threshold: f64,
    /// Maximum execution time (seconds)
    pub max_execution_time_secs: u64,
    /// Enable MEV protection
    pub mev_protection: bool,
}

impl Default for ArbitrageConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_profit_usd: U256::from(10_000_000u64), // $10
            max_profit_usd: U256::from(1_000_000_000_000u64), // $1M
            confidence_threshold: 0.8, // 80%
            max_execution_time_secs: 300, // 5 minutes
            mev_protection: true,
        }
    }
}

/// State management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateConfig {
    /// Enable persistent storage
    pub persistent_storage: bool,
    /// Snapshot interval (seconds)
    pub snapshot_interval_secs: u64,
    /// Maximum snapshots to keep
    pub max_snapshots: usize,
    /// Compression enabled
    pub compression: bool,
    /// Enable state validation
    pub validate_state: bool,
}

impl Default for StateConfig {
    fn default() -> Self {
        Self {
            persistent_storage: true,
            snapshot_interval_secs: 300, // 5 minutes
            max_snapshots: 100,
            compression: true,
            validate_state: true,
        }
    }
}

/// Event system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventConfig {
    /// Enable event system
    pub enabled: bool,
    /// Event queue size
    pub queue_size: usize,
    /// Event retention time (seconds)
    pub retention_secs: u64,
    /// Enable event persistence
    pub persistent: bool,
    /// Event batching
    pub batch_size: usize,
}

impl Default for EventConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            queue_size: 10_000,
            retention_secs: 86_400, // 24 hours
            persistent: false,
            batch_size: 100,
        }
    }
}

/// Integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationConfig {
    /// Evolution Core integration
    pub evolution_core: EvolutionCoreConfig,
    /// GPU Swarm integration
    pub gpu_swarm: GpuSwarmConfig,
    /// External API configurations
    pub apis: ApiConfigs,
    /// Oracle configurations
    pub oracles: OracleConfigs,
}

impl Default for IntegrationConfig {
    fn default() -> Self {
        Self {
            evolution_core: EvolutionCoreConfig::default(),
            gpu_swarm: GpuSwarmConfig::default(),
            apis: ApiConfigs::default(),
            oracles: OracleConfigs::default(),
        }
    }
}

/// Evolution Core integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionCoreConfig {
    /// Enable Evolution Core integration
    pub enabled: bool,
    /// Evolution Core endpoint
    pub endpoint: String,
    /// API key for authentication
    pub api_key: Option<String>,
    /// Request timeout (seconds)
    pub timeout_secs: u64,
}

impl Default for EvolutionCoreConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            endpoint: "http://localhost:8081".to_string(),
            api_key: None,
            timeout_secs: 30,
        }
    }
}

/// GPU Swarm integration configuration
