//! ChronosFlash configuration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use crate::types::ChainId;

/// Main configuration for ChronosFlash oracle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChronosConfig {
    /// Maximum chains to monitor simultaneously
    pub max_chains: usize,
    /// Target latency in milliseconds
    pub target_latency_ms: u64,
    /// Maximum time-warp advantage in milliseconds
    pub max_timewarp_ms: u64,
    /// Intent prediction confidence threshold (0.0 - 1.0)
    pub prediction_threshold: f64,
    /// Maximum slippage tolerance in basis points
    pub max_slippage_bps: u32,
    /// Route expiry duration
    pub route_expiry: Duration,
    /// Checkpoint retention duration
    pub checkpoint_retention: Duration,
    /// Per-chain configurations
    pub chains: HashMap<ChainId, ChainConfig>,
    /// Mempool scanner config
    pub mempool: MempoolConfig,
    /// Quantum router config
    pub router: RouterConfig,
    /// AI predictor config
    pub predictor: PredictorConfig,
}

impl Default for ChronosConfig {
    fn default() -> Self {
        Self {
            max_chains: 150,
            target_latency_ms: 50,
            max_timewarp_ms: 400,
            prediction_threshold: 0.85,
            max_slippage_bps: 50, // 0.5%
            route_expiry: Duration::from_secs(30),
            checkpoint_retention: Duration::from_secs(300),
            chains: HashMap::new(),
            mempool: MempoolConfig::default(),
            router: RouterConfig::default(),
            predictor: PredictorConfig::default(),
        }
    }
}

/// Per-chain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    pub chain_id: ChainId,
    pub name: String,
    pub rpc_endpoints: Vec<String>,
    pub ws_endpoints: Vec<String>,
    pub mempool_endpoint: Option<String>,
    pub block_time_ms: u64,
    pub confirmation_blocks: u32,
    pub gas_price_multiplier: f64,
    pub max_gas_price: u128,
    pub priority: u8,
    pub enabled: bool,
}

impl ChainConfig {
    /// Create config for Ethereum mainnet
    pub fn ethereum() -> Self {
        Self {
            chain_id: 1,
            name: "Ethereum".to_string(),
            rpc_endpoints: vec!["https://eth.llamarpc.com".to_string()],
            ws_endpoints: vec!["wss://eth.llamarpc.com".to_string()],
            mempool_endpoint: Some("wss://mempool.eth.llamarpc.com".to_string()),
            block_time_ms: 12000,
            confirmation_blocks: 2,
            gas_price_multiplier: 1.1,
            max_gas_price: 500_000_000_000, // 500 gwei
            priority: 10,
            enabled: true,
        }
    }

    /// Create config for Polygon
    pub fn polygon() -> Self {
        Self {
            chain_id: 137,
            name: "Polygon".to_string(),
            rpc_endpoints: vec!["https://polygon.llamarpc.com".to_string()],
            ws_endpoints: vec!["wss://polygon.llamarpc.com".to_string()],
            mempool_endpoint: Some("wss://mempool.polygon.llamarpc.com".to_string()),
            block_time_ms: 2000,
            confirmation_blocks: 5,
            gas_price_multiplier: 1.2,
            max_gas_price: 1_000_000_000_000, // 1000 gwei
            priority: 8,
            enabled: true,
        }
    }

    /// Create config for Arbitrum
    pub fn arbitrum() -> Self {
        Self {
            chain_id: 42161,
            name: "Arbitrum".to_string(),
            rpc_endpoints: vec!["https://arbitrum.llamarpc.com".to_string()],
            ws_endpoints: vec!["wss://arbitrum.llamarpc.com".to_string()],
            mempool_endpoint: Some("wss://mempool.arbitrum.llamarpc.com".to_string()),
            block_time_ms: 250,
            confirmation_blocks: 1,
            gas_price_multiplier: 1.1,
            max_gas_price: 10_000_000_000, // 10 gwei
            priority: 9,
            enabled: true,
        }
    }

    /// Create config for Solana
    pub fn solana() -> Self {
        Self {
            chain_id: 1399811149, // Solana chain ID
            name: "Solana".to_string(),
            rpc_endpoints: vec!["https://api.mainnet-beta.solana.com".to_string()],
            ws_endpoints: vec!["wss://api.mainnet-beta.solana.com".to_string()],
            mempool_endpoint: Some("wss://api.mainnet-beta.solana.com".to_string()),
            block_time_ms: 400,
            confirmation_blocks: 1,
            gas_price_multiplier: 1.0,
            max_gas_price: 0, // Solana uses compute units
            priority: 10,
            enabled: true,
        }
    }

    /// Create config for Atlas Sphere L1
    pub fn atlas_sphere() -> Self {
        Self {
            chain_id: 0x41544C53, // "ATLS" in hex
            name: "Atlas Sphere".to_string(),
            rpc_endpoints: vec!["https://rpc.atlas-sphere.io".to_string()],
            ws_endpoints: vec!["wss://ws.atlas-sphere.io".to_string()],
            mempool_endpoint: Some("wss://mempool.atlas-sphere.io".to_string()),
            block_time_ms: 6000,
            confirmation_blocks: 1,
            gas_price_multiplier: 1.0,
            max_gas_price: 100_000_000_000,
            priority: 10,
            enabled: true,
        }
    }
}

/// Mempool scanner configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MempoolConfig {
    /// Scan interval in milliseconds
    pub scan_interval_ms: u64,
    /// Maximum pending transactions to track per chain
    pub max_pending_per_chain: usize,
    /// Transaction pattern matching depth
    pub pattern_depth: u8,
    /// DEX contract addresses to monitor
    pub dex_contracts: Vec<DexContract>,
    /// Enable intent prediction
    pub enable_prediction: bool,
    /// Prediction lookahead in blocks
    pub prediction_lookahead: u32,
}

impl Default for MempoolConfig {
    fn default() -> Self {
        Self {
            scan_interval_ms: 10,
            max_pending_per_chain: 10000,
            pattern_depth: 3,
            dex_contracts: vec![
                DexContract::uniswap_v3(),
                DexContract::sushiswap(),
                DexContract::curve(),
            ],
            enable_prediction: true,
            prediction_lookahead: 3,
        }
    }
}

/// Known DEX contract configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexContract {
    pub chain_id: ChainId,
    pub name: String,
    pub router_address: [u8; 20],
    pub swap_selector: [u8; 4],
    pub protocol_type: ProtocolType,
}

impl DexContract {
    pub fn uniswap_v3() -> Self {
        Self {
            chain_id: 1,
            name: "Uniswap V3".to_string(),
            router_address: hex_to_array("E592427A0AEce92De3Edee1F18E0157C05861564"),
            swap_selector: [0xc0, 0x4b, 0x8d, 0x59], // exactInputSingle
            protocol_type: ProtocolType::UniswapV3,
        }
    }

    pub fn sushiswap() -> Self {
        Self {
            chain_id: 1,
            name: "SushiSwap".to_string(),
            router_address: hex_to_array("d9e1cE17f2641f24aE83637ab66a2cca9C378B9F"),
            swap_selector: [0x38, 0xed, 0x17, 0x39], // swapExactTokensForTokens
            protocol_type: ProtocolType::UniswapV2,
        }
    }

    pub fn curve() -> Self {
        Self {
            chain_id: 1,
            name: "Curve".to_string(),
            router_address: hex_to_array("99a58482BD75cbab83b27EC03CA68fF489b5788f"),
            swap_selector: [0x3d, 0xf0, 0x21, 0x24], // exchange
            protocol_type: ProtocolType::Curve,
        }
    }
}

/// DEX protocol type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProtocolType {
    UniswapV2,
    UniswapV3,
    Curve,
    Balancer,
    DODO,
    Raydium,
    Orca,
    Jupiter,
    AtlasDex,
}

/// Quantum router configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterConfig {
    /// Maximum route hops
    pub max_hops: u8,
    /// Maximum routes to compute per intent
    pub max_routes: usize,
    /// Route optimization timeout
    pub optimization_timeout: Duration,
    /// Enable cross-chain routes
    pub cross_chain_enabled: bool,
    /// Evolution iterations for quantum optimization
    pub evolution_iterations: u32,
    /// Population size for genetic algorithm
    pub population_size: usize,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            max_hops: 5,
            max_routes: 10,
            optimization_timeout: Duration::from_millis(100),
            cross_chain_enabled: true,
            evolution_iterations: 50,
            population_size: 100,
        }
    }
}

/// AI predictor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictorConfig {
    /// Model type for intent prediction
    pub model_type: PredictorModel,
    /// Confidence threshold for predictions
    pub confidence_threshold: f64,
    /// Historical data window in blocks
    pub history_window: u32,
    /// Feature extraction depth
    pub feature_depth: u8,
    /// Enable GPU acceleration
    pub gpu_acceleration: bool,
    /// Swarm size for distributed prediction
    pub swarm_size: usize,
}

impl Default for PredictorConfig {
    fn default() -> Self {
        Self {
            model_type: PredictorModel::Hybrid,
            confidence_threshold: 0.85,
            history_window: 1000,
            feature_depth: 5,
            gpu_acceleration: true,
            swarm_size: 8,
        }
    }
}

/// Predictor model type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PredictorModel {
    /// Simple pattern matching
    Pattern,
    /// Neural network based
    Neural,
    /// Ensemble model
    Ensemble,
    /// Hybrid quantum-classical
    Hybrid,
}

// Helper function
fn hex_to_array(hex: &str) -> [u8; 20] {
    let bytes = hex::decode(hex).unwrap_or_else(|_| vec![0u8; 20]);
    let mut arr = [0u8; 20];
    arr.copy_from_slice(&bytes[..20]);
    arr
}
