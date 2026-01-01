//! Configuration for the Quantum Swarm Executor

use crate::{LogLevel, MutationRate, OperatingMode, SwarmSizeMode};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Main configuration for the Quantum Swarm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumSwarmConfig {
    /// Operating mode
    pub mode: OperatingMode,
    /// Logging level
    pub log_level: LogLevel,
    /// Quantum configuration
    pub quantum: QuantumConfig,
    /// Classical ML configuration
    pub classical: ClassicalConfig,
    /// Evolution configuration
    pub evolution: EvolutionConfig,
    /// Arena configuration
    pub arena: ArenaConfig,
    /// Compute fabric configuration
    pub compute: ComputeFabricConfig,
    /// Strategy configuration
    pub strategy: StrategyConfig,
    /// Financial constraints
    pub financial: FinancialConfig,
}

impl Default for QuantumSwarmConfig {
    fn default() -> Self {
        Self {
            mode: OperatingMode::SuperYolo,
            log_level: LogLevel::Full,
            quantum: QuantumConfig::default(),
            classical: ClassicalConfig::default(),
            evolution: EvolutionConfig::default(),
            arena: ArenaConfig::default(),
            compute: ComputeFabricConfig::default(),
            strategy: StrategyConfig::default(),
            financial: FinancialConfig::default(),
        }
    }
}

/// Quantum computing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumConfig {
    /// Enable local quantum simulation
    pub enable_local_sim: bool,
    /// Enable external QPU access
    pub enable_external_qpu: bool,
    /// Maximum circuit depth
    pub max_circuit_depth: usize,
    /// Maximum qubits
    pub max_qubits: usize,
    /// QAOA layers
    pub qaoa_layers: usize,
    /// VQE max iterations
    pub vqe_max_iterations: usize,
    /// Annealing schedule
    pub annealing_schedule: AnnealingSchedule,
    /// Simulator backend
    pub simulator_backend: SimulatorBackend,
    /// Shot count for measurements
    pub shots: usize,
    /// Minimum expected value ratio for QPU requests
    pub min_qpu_ev_ratio: f64,
    /// IBM Quantum API key
    pub ibm_api_key: Option<String>,
    /// AWS Braket region
    pub braket_region: Option<String>,
}

impl Default for QuantumConfig {
    fn default() -> Self {
        Self {
            enable_local_sim: true,
            enable_external_qpu: false,
            max_circuit_depth: 100,
            max_qubits: 20,
            qaoa_layers: 3,
            vqe_max_iterations: 1000,
            annealing_schedule: AnnealingSchedule::Linear,
            simulator_backend: SimulatorBackend::Statevector,
            shots: 1024,
            min_qpu_ev_ratio: 1.5,
            ibm_api_key: None,
            braket_region: None,
        }
    }
}

/// Annealing schedule for QUBO
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnnealingSchedule {
    Linear,
    Exponential,
    Logarithmic,
    Adaptive,
}

/// Quantum simulator backend
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimulatorBackend {
    /// Full statevector simulation
    Statevector,
    /// Matrix product state (for large circuits)
    Mps,
    /// Density matrix (for noise modeling)
    DensityMatrix,
    /// GPU-accelerated
    GpuAccelerated,
}

/// Classical ML configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassicalConfig {
    /// Enable LSTM models
    pub enable_lstm: bool,
    /// Enable ensemble methods
    pub enable_ensemble: bool,
    /// LSTM hidden size
    pub lstm_hidden_size: usize,
    /// LSTM layers
    pub lstm_layers: usize,
    /// Sequence length for time series
    pub sequence_length: usize,
    /// Batch size for training
    pub batch_size: usize,
    /// Learning rate
    pub learning_rate: f64,
    /// Ensemble size
    pub ensemble_size: usize,
}

impl Default for ClassicalConfig {
    fn default() -> Self {
        Self {
            enable_lstm: true,
            enable_ensemble: true,
            lstm_hidden_size: 128,
            lstm_layers: 2,
            sequence_length: 100,
            batch_size: 32,
            learning_rate: 0.001,
            ensemble_size: 5,
        }
    }
}

/// Evolution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionConfig {
    /// Population size mode
    pub population_size: SwarmSizeMode,
    /// Mutation rate mode
    pub mutation_rate: MutationRate,
    /// Crossover probability
    pub crossover_prob: f64,
    /// Elite count (survivors)
    pub elite_count: usize,
    /// Selection method
    pub selection_method: SelectionMethod,
    /// Max generations
    pub max_generations: usize,
    /// Convergence threshold
    pub convergence_threshold: f64,
    /// Enable speciation
    pub enable_speciation: bool,
    /// Species compatibility threshold
    pub compatibility_threshold: f64,
}

impl Default for EvolutionConfig {
    fn default() -> Self {
        Self {
            population_size: SwarmSizeMode::Dynamic,
            mutation_rate: MutationRate::Adaptive,
            crossover_prob: 0.7,
            elite_count: 5,
            selection_method: SelectionMethod::Tournament,
            max_generations: 1000,
            convergence_threshold: 0.001,
            enable_speciation: true,
            compatibility_threshold: 3.0,
        }
    }
}

/// Selection method for evolution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionMethod {
    /// Tournament selection
    Tournament,
    /// Roulette wheel
    RouletteWheel,
    /// Rank-based
    Rank,
    /// NSGA-II (multi-objective)
    Nsga2,
}

/// Arena configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArenaConfig {
    /// Maximum strategies in arena
    pub max_strategies: usize,
    /// Tournament interval
    pub tournament_interval: Duration,
    /// Minimum participants for tournament
    pub min_participants: usize,
    /// Evaluation timeout
    pub evaluation_timeout: Duration,
    /// Kill threshold (fitness percentile)
    pub kill_threshold: f64,
    /// Archive losers before killing
    pub archive_losers: bool,
}

impl Default for ArenaConfig {
    fn default() -> Self {
        Self {
            max_strategies: 1000,
            tournament_interval: Duration::from_secs(60),
            min_participants: 10,
            evaluation_timeout: Duration::from_secs(30),
            kill_threshold: 0.1, // Kill bottom 10%
            archive_losers: true,
        }
    }
}

/// Compute fabric configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeFabricConfig {
    /// Maximum parallel classical jobs
    pub max_classical_parallel: usize,
    /// Maximum parallel quantum jobs
    pub max_quantum_parallel: usize,
    /// GPU memory limit (MB)
    pub gpu_memory_limit_mb: usize,
    /// Classical timeout
    pub classical_timeout: Duration,
    /// Quantum timeout
    pub quantum_timeout: Duration,
    /// Auto-route selection
    pub auto_route: bool,
    /// Latency threshold for quantum upgrade (ms)
    pub quantum_upgrade_threshold_ms: u64,
}

impl Default for ComputeFabricConfig {
    fn default() -> Self {
        Self {
            max_classical_parallel: 32,
            max_quantum_parallel: 4,
            gpu_memory_limit_mb: 8192,
            classical_timeout: Duration::from_secs(60),
            quantum_timeout: Duration::from_secs(300),
            auto_route: true,
            quantum_upgrade_threshold_ms: 100,
        }
    }
}

/// Strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    /// Enable arbitrage strategies
    pub enable_arbitrage: bool,
    /// Enable portfolio optimization
    pub enable_portfolio: bool,
    /// Enable market making
    pub enable_market_making: bool,
    /// Enable MEV extraction
    pub enable_mev: bool,
    /// Enable liquidation hunting
    pub enable_liquidation: bool,
    /// Maximum concurrent strategies
    pub max_concurrent: usize,
    /// Strategy evaluation window
    pub evaluation_window: Duration,
}

impl Default for StrategyConfig {
    fn default() -> Self {
        Self {
            enable_arbitrage: true,
            enable_portfolio: true,
            enable_market_making: true,
            enable_mev: true,
            enable_liquidation: true,
            max_concurrent: 100,
            evaluation_window: Duration::from_secs(3600),
        }
    }
}

/// Financial constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialConfig {
    /// Maximum latency (ms)
    pub max_latency_ms: u64,
    /// Minimum profit threshold
    pub min_profit: f64,
    /// Maximum slippage (bps)
    pub max_slippage_bps: u32,
    /// Maximum position size
    pub max_position_size: f64,
    /// Risk budget
    pub risk_budget: f64,
    /// Maximum drawdown tolerance
    pub max_drawdown: f64,
    /// Gas price limit (gwei)
    pub max_gas_price_gwei: u64,
}

impl Default for FinancialConfig {
    fn default() -> Self {
        Self {
            max_latency_ms: 150,
            min_profit: 0.0,
            max_slippage_bps: 100,
            max_position_size: 100000.0,
            risk_budget: 0.1,
            max_drawdown: 0.2,
            max_gas_price_gwei: 100,
        }
    }
}
