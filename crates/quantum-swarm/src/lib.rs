//! # Quantum Swarm Executor
//!
//! Autonomous hybrid quantum-classical compute intelligence for Atlas Sphere.
//!
//! ## Overview
//!
//! The Quantum Swarm Executor is a relentless optimization engine that combines:
//! - **Quantum Computing**: QAOA, VQE, QUBO annealing for hard optimization problems
//! - **Classical ML**: LSTMs, transformers, evolutionary algorithms
//! - **Swarm Intelligence**: Distributed competitive agents fighting for survival
//! - **Financial Strategy**: Arbitrage routing, portfolio optimization, risk modeling
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────────────┐
//! │                    QUANTUM SWARM EXECUTOR                                │
//! ├──────────────────────────────────────────────────────────────────────────┤
//! │                                                                          │
//! │  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐             │
//! │  │  QUANTUM LAYER │  │  CLASSICAL ML  │  │  SWARM ARENA   │             │
//! │  │  ─────────────  │  │  ─────────────  │  │  ─────────────  │             │
//! │  │  • Qiskit       │  │  • LSTMs       │  │  • Tournament  │             │
//! │  │  • PennyLane    │  │  • Transformers│  │  • Evolution   │             │
//! │  │  • D-Wave QUBO  │  │  • XGBoost     │  │  • Mutation    │             │
//! │  │  • Cirq         │  │  • Ensemble    │  │  • Selection   │             │
//! │  └───────┬────────┘  └───────┬────────┘  └───────┬────────┘             │
//! │          │                   │                   │                      │
//! │          └───────────────────┼───────────────────┘                      │
//! │                              ▼                                          │
//! │  ┌────────────────────────────────────────────────────────────────┐    │
//! │  │                    COMPUTE FABRIC ROUTER                       │    │
//! │  │  • Classical CPU (trivial math)                                │    │
//! │  │  • GPU Swarm (medium complexity)                               │    │
//! │  │  • Local Quantum Sim (quantum advantage candidates)            │    │
//! │  │  • External QPU (IBM/Braket/Cirq - when profitable)            │    │
//! │  └───────────────────────────┬────────────────────────────────────┘    │
//! │                              ▼                                          │
//! │  ┌────────────────────────────────────────────────────────────────┐    │
//! │  │                     STRATEGY ENGINES                           │    │
//! │  │  • Arbitrage Router    • Portfolio Optimizer                   │    │
//! │  │  • Risk Modeler        • MEV Extractor                         │    │
//! │  │  • Liquidation Hunter  • Market Maker                          │    │
//! │  └───────────────────────────┬────────────────────────────────────┘    │
//! │                              ▼                                          │
//! │  ┌────────────────────────────────────────────────────────────────┐    │
//! │  │                    X3 CONTRACT EMITTER                         │    │
//! │  │  • Generate optimized strategies as executable X3 code         │    │
//! │  │  • Deploy across EVM + SVM atomically                          │    │
//! │  │  • Emit Quantum Compute Credit accounting                      │    │
//! │  └────────────────────────────────────────────────────────────────┘    │
//! │                                                                          │
//! └──────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Operating Principles
//!
//! 1. **Speed First**: Always choose the fastest compute path
//!    - Classical math for trivial operations
//!    - Local quantum simulators for medium difficulty
//!    - External QPU only when expected value > cost
//!
//! 2. **Never Trust Single Models**: Spawn competitive swarms
//!    - Multiple algorithm families compete simultaneously
//!    - Only profitable strategies survive
//!
//! 3. **Disposable Algorithms**: Every strategy is killable
//!    - Underperformers get terminated, mutated, or quantized
//!    - Only profit survives
//!
//! 4. **Profitable QPU Requests**: External quantum time requires justification
//!    - Must prove improvement in PnL, risk, latency, or optimization depth
//!
//! 5. **Executable Output**: Everything compiles to X3
//!    - All strategies deployable across connected chains

// Core modules
pub mod config;
pub mod error;
pub mod types;

// Quantum computing layer
pub mod quantum;

// Evolution and competition
pub mod arena;
pub mod evolution;

// Strategy engines
pub mod strategy;

// Main executor
pub mod executor;

// Re-exports for convenience
pub use config::QuantumSwarmConfig;
pub use error::{SwarmError, SwarmResult};
pub use types::*;

// Quantum re-exports
pub use quantum::{
    AnnealingResult, Ansatz, BackendCapabilities, CircuitBuilder, Gate, IsingModel, LocalSimulator,
    QaoaConfig, QaoaOptimizer, QaoaResult, QuantumBackend, QuantumCircuit, QuboMatrix, QuboSolver,
    VqeConfig, VqeOptimizer, VqeResult,
};

// Evolution re-exports
pub use evolution::{
    EvolutionConfig, EvolutionEngine, EvolutionStats, Gene, GeneType, Genome, Mutation,
    MutationOperator, Population, Selection, SelectionOperator, Species,
};

// Arena re-exports
pub use arena::{
    Archive, ArchivedStrategy, Arena, ArenaConfig, ArenaStats, Combatant, CombatantStatus,
    MatchResult, Tournament, TournamentResult, TournamentType,
};

// Strategy re-exports
pub use strategy::{
    ArbitrageConfig, ArbitrageOpportunity, ArbitrageStrategy, MarketData, MarketMakingConfig,
    MarketMakingStrategy, PortfolioAllocation, PortfolioConfig, PortfolioStrategy, Signal,
    Strategy, StrategyMetrics, StrategyType, TrendConfig, TrendSignal, TrendStrategy,
};

// Executor re-exports
pub use executor::{
    AsyncQuantumSwarmExecutor, ExecutorMode, ExecutorState, ExecutorStats, QuantumSwarmExecutor,
    StepResult,
};

/// Bootstrap command for the Quantum Swarm
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BootstrapCommand {
    /// Primary objective
    pub objective: String,
    /// Operating constraints
    pub constraints: Vec<String>,
    /// Available tools
    pub tools: Vec<String>,
    /// Allowed behaviors
    pub behaviors: Vec<String>,
    /// Expected outputs
    pub outputs: Vec<String>,
    /// Priority rules for compute selection
    pub priority_rules: Vec<String>,
    /// Swarm size mode
    pub swarm_size: SwarmSizeMode,
    /// Mutation rate configuration
    pub mutation_rate: MutationRate,
    /// Evaluation metric
    pub evaluation_metric: String,
    /// Log verbosity
    pub logs: LogLevel,
    /// Operating mode
    pub mode: OperatingMode,
}

impl Default for BootstrapCommand {
    fn default() -> Self {
        Self {
            objective: "maximize pnl & arbitrage throughput".to_string(),
            constraints: vec![
                "latency <= 150ms".to_string(),
                "profit > 0".to_string(),
                "minimize slippage".to_string(),
            ],
            tools: vec![
                "Qiskit".to_string(),
                "PennyLane".to_string(),
                "D-Wave Ocean".to_string(),
                "Cirq".to_string(),
                "local simulators".to_string(),
                "GPUs".to_string(),
                "X3 compiler".to_string(),
            ],
            behaviors: vec![
                "evolve".to_string(),
                "mutate".to_string(),
                "compete".to_string(),
                "kill losers".to_string(),
                "deploy winners".to_string(),
            ],
            outputs: vec![
                "strategies".to_string(),
                "X3 contracts".to_string(),
                "routes".to_string(),
                "quantum circuits".to_string(),
                "QPU jobs".to_string(),
                "risk profiles".to_string(),
            ],
            priority_rules: vec![
                "use classical first".to_string(),
                "upgrade to quantum when beneficial".to_string(),
                "use real QPU only when profitable".to_string(),
            ],
            swarm_size: SwarmSizeMode::Dynamic,
            mutation_rate: MutationRate::Adaptive,
            evaluation_metric: "PnL + risk-adjusted return + execution time".to_string(),
            logs: LogLevel::Full,
            mode: OperatingMode::SuperYolo,
        }
    }
}

/// Swarm size configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SwarmSizeMode {
    /// Fixed number of agents
    Fixed(usize),
    /// Dynamic scaling based on load
    Dynamic,
    /// Aggressive scaling for maximum throughput
    Aggressive,
}

/// Mutation rate configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MutationRate {
    /// Fixed mutation rate
    Fixed(u8), // percentage 0-100
    /// Adaptive based on performance
    Adaptive,
    /// Aggressive mutation for rapid evolution
    Aggressive,
}

/// Log verbosity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LogLevel {
    /// Minimal logging
    Minimal,
    /// Standard logging
    Standard,
    /// Full detail logging
    Full,
}

/// Operating mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OperatingMode {
    /// Conservative mode - prioritize safety
    Conservative,
    /// Standard mode - balanced approach
    Standard,
    /// Aggressive mode - prioritize speed
    Aggressive,
    /// YOLO mode - maximum risk, maximum reward
    Yolo,
    /// Super YOLO mode - no fear, no hesitation, only optimization
    SuperYolo,
}

/// Protocol version
pub const PROTOCOL_VERSION: u32 = 1;

/// Maximum strategies in arena
pub const MAX_ARENA_STRATEGIES: usize = 1000;

/// Default quantum circuit depth limit
pub const DEFAULT_CIRCUIT_DEPTH: usize = 100;

/// Default QAOA layers
pub const DEFAULT_QAOA_LAYERS: usize = 3;

/// Minimum expected value ratio for QPU requests
pub const MIN_QPU_EV_RATIO: f64 = 1.5; // Must expect 50% improvement to justify QPU cost
