//! Core types for the Quantum Swarm Executor

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for strategies
pub type StrategyId = Uuid;

/// Unique identifier for agents
pub type AgentId = Uuid;

/// Unique identifier for tournaments
pub type TournamentId = Uuid;

/// Unique identifier for quantum jobs
pub type QuantumJobId = Uuid;

/// Asset identifier
pub type AssetId = String;

/// Chain identifier
pub type ChainId = u64;

/// Balance in smallest units
pub type Balance = u128;

/// Price in fixed-point representation (18 decimals)
pub type Price = u128;

/// Timestamp in milliseconds
pub type Timestamp = u64;

/// Hash type (32 bytes)
pub type Hash = [u8; 32];

/// Swarm message for inter-agent communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SwarmMessage {
    /// Strategy submission
    StrategySubmit(StrategySubmission),
    /// Strategy evaluation result
    StrategyResult(StrategyEvaluation),
    /// Tournament announcement
    TournamentAnnounce(TournamentAnnouncement),
    /// Tournament result
    TournamentResult(TournamentOutcome),
    /// Quantum job request
    QuantumJobRequest(QuantumJob),
    /// Quantum job result
    QuantumJobResult(QuantumJobOutput),
    /// Evolution cycle
    EvolutionCycle(EvolutionEvent),
    /// Metrics update
    MetricsUpdate(MetricsSnapshot),
    /// Kill signal
    Kill(KillSignal),
}

/// Strategy submission to the arena
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategySubmission {
    pub id: StrategyId,
    pub agent_id: AgentId,
    pub strategy_type: StrategyTypeId,
    pub parameters: HashMap<String, f64>,
    pub code: Option<String>,
    pub submitted_at: Timestamp,
}

/// Strategy evaluation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyEvaluation {
    pub strategy_id: StrategyId,
    pub pnl: f64,
    pub risk_score: f64,
    pub execution_time_ms: u64,
    pub gas_used: u64,
    pub slippage: f64,
    pub success_rate: f64,
    pub evaluated_at: Timestamp,
}

/// Strategy type identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StrategyTypeId {
    /// Arbitrage routing
    Arbitrage,
    /// Portfolio optimization
    Portfolio,
    /// Market making
    MarketMaking,
    /// Liquidation hunting
    Liquidation,
    /// MEV extraction
    MevExtraction,
    /// Risk hedging
    RiskHedge,
    /// Yield farming
    YieldFarm,
    /// Custom strategy
    Custom(u32),
}

/// Tournament announcement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TournamentAnnouncement {
    pub id: TournamentId,
    pub objective: String,
    pub constraints: Vec<Constraint>,
    pub reward_pool: Balance,
    pub deadline: Timestamp,
    pub min_participants: usize,
}

/// Tournament outcome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TournamentOutcome {
    pub id: TournamentId,
    pub winner: StrategyId,
    pub ranking: Vec<(StrategyId, f64)>,
    pub total_pnl: f64,
    pub concluded_at: Timestamp,
}

/// Constraint for optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    pub name: String,
    pub constraint_type: ConstraintType,
    pub value: f64,
}

/// Type of constraint
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstraintType {
    /// Maximum value constraint
    Max,
    /// Minimum value constraint
    Min,
    /// Equality constraint
    Equal,
    /// Less than or equal
    LessOrEqual,
    /// Greater than or equal
    GreaterOrEqual,
}

/// Quantum job request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumJob {
    pub id: QuantumJobId,
    pub job_type: QuantumJobType,
    pub circuit: Option<SerializableCircuit>,
    pub parameters: HashMap<String, f64>,
    pub backend_preference: BackendPreference,
    pub expected_value: f64,
    pub max_cost: f64,
    pub timeout_ms: u64,
}

/// Type of quantum job
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuantumJobType {
    /// QAOA optimization
    Qaoa,
    /// VQE ground state
    Vqe,
    /// QUBO annealing
    QuboAnneal,
    /// Custom circuit
    CustomCircuit,
    /// Hybrid quantum-classical
    Hybrid,
}

/// Backend preference for quantum execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackendPreference {
    /// Local simulator only
    LocalOnly,
    /// Prefer local, use external if beneficial
    PreferLocal,
    /// Any available backend
    Any,
    /// External QPU only
    ExternalOnly,
    /// Specific backend
    Specific(ExternalBackend),
}

/// External quantum backends
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExternalBackend {
    /// IBM Quantum
    IbmQuantum,
    /// Amazon Braket
    AmazonBraket,
    /// Google Cirq
    GoogleCirq,
    /// D-Wave
    DWave,
    /// IonQ
    IonQ,
    /// Rigetti
    Rigetti,
}

/// Serializable quantum circuit representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableCircuit {
    pub num_qubits: usize,
    pub gates: Vec<SerializableGate>,
    pub measurements: Vec<usize>,
}

/// Serializable quantum gate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableGate {
    pub gate_type: GateType,
    pub qubits: Vec<usize>,
    pub parameters: Vec<f64>,
}

/// Quantum gate types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GateType {
    // Single-qubit gates
    H,  // Hadamard
    X,  // Pauli-X
    Y,  // Pauli-Y
    Z,  // Pauli-Z
    S,  // S gate
    T,  // T gate
    Rx, // Rotation around X
    Ry, // Rotation around Y
    Rz, // Rotation around Z
    U,  // Universal single-qubit

    // Two-qubit gates
    Cnot, // Controlled-NOT
    Cz,   // Controlled-Z
    Swap, // Swap

    // Multi-qubit gates
    Ccx,   // Toffoli
    Cswap, // Fredkin

    // Parametric gates
    Rxx, // XX rotation
    Ryy, // YY rotation
    Rzz, // ZZ rotation
}

/// Quantum job output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumJobOutput {
    pub job_id: QuantumJobId,
    pub success: bool,
    pub result: Option<QuantumResult>,
    pub backend_used: String,
    pub execution_time_ms: u64,
    pub cost: f64,
    pub error: Option<String>,
}

/// Quantum computation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumResult {
    /// Optimal parameters found
    pub optimal_params: Vec<f64>,
    /// Optimal energy/cost
    pub optimal_value: f64,
    /// Measurement counts
    pub counts: HashMap<String, u64>,
    /// Statevector (if available)
    pub statevector: Option<Vec<num_complex::Complex64>>,
    /// Convergence history
    pub history: Vec<f64>,
}

/// Evolution event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionEvent {
    pub generation: u64,
    pub population_size: usize,
    pub best_fitness: f64,
    pub mean_fitness: f64,
    pub mutation_count: usize,
    pub crossover_count: usize,
    pub extinction_count: usize,
}

/// Metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp: Timestamp,
    pub total_strategies: usize,
    pub active_strategies: usize,
    pub total_pnl: f64,
    pub total_volume: Balance,
    pub quantum_jobs_completed: u64,
    pub quantum_jobs_pending: u64,
    pub tournaments_run: u64,
    pub evolution_generations: u64,
}

/// Kill signal for terminating strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KillSignal {
    pub target: KillTarget,
    pub reason: String,
    pub archive: bool,
}

/// Target for kill signal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KillTarget {
    /// Kill specific strategy
    Strategy(StrategyId),
    /// Kill all strategies of a type
    StrategyType(StrategyTypeId),
    /// Kill agent
    Agent(AgentId),
    /// Kill all with fitness below threshold
    BelowFitness(f64),
}

/// Compute path selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComputePath {
    /// Classical CPU computation
    Classical,
    /// GPU accelerated computation
    Gpu,
    /// Local quantum simulator
    LocalQuantumSim,
    /// External QPU
    ExternalQpu(ExternalBackend),
}

/// Financial position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub asset: AssetId,
    pub chain: ChainId,
    pub amount: Balance,
    pub entry_price: Price,
    pub current_price: Price,
    pub unrealized_pnl: f64,
}

/// Trade route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRoute {
    pub hops: Vec<TradeHop>,
    pub expected_output: Balance,
    pub expected_slippage: f64,
    pub gas_estimate: u64,
    pub total_time_ms: u64,
}

/// Single hop in a trade route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeHop {
    pub chain: ChainId,
    pub protocol: String,
    pub pool: String,
    pub input_asset: AssetId,
    pub output_asset: AssetId,
    pub input_amount: Balance,
    pub expected_output: Balance,
}

/// Risk profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskProfile {
    /// Value at Risk (95%)
    pub var_95: f64,
    /// Value at Risk (99%)
    pub var_99: f64,
    /// Expected shortfall
    pub expected_shortfall: f64,
    /// Maximum drawdown
    pub max_drawdown: f64,
    /// Sharpe ratio
    pub sharpe_ratio: f64,
    /// Sortino ratio
    pub sortino_ratio: f64,
    /// Beta to market
    pub beta: f64,
    /// Correlation matrix eigenvalues
    pub risk_eigenvalues: Vec<f64>,
}

impl Default for RiskProfile {
    fn default() -> Self {
        Self {
            var_95: 0.0,
            var_99: 0.0,
            expected_shortfall: 0.0,
            max_drawdown: 0.0,
            sharpe_ratio: 0.0,
            sortino_ratio: 0.0,
            beta: 1.0,
            risk_eigenvalues: Vec::new(),
        }
    }
}

/// Optimization problem for quantum solvers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationProblem {
    /// Problem type
    pub problem_type: ProblemType,
    /// Number of variables
    pub num_variables: usize,
    /// Objective function coefficients
    pub objective: Vec<f64>,
    /// Quadratic terms (for QUBO)
    pub quadratic: Option<Vec<(usize, usize, f64)>>,
    /// Linear constraints
    pub constraints: Vec<LinearConstraint>,
    /// Variable bounds
    pub bounds: Vec<(f64, f64)>,
}

/// Problem type for optimization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProblemType {
    /// Quadratic Unconstrained Binary Optimization
    Qubo,
    /// Mixed Integer Linear Programming
    Milp,
    /// Continuous optimization
    Continuous,
    /// Combinatorial
    Combinatorial,
}

/// Linear constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinearConstraint {
    pub coefficients: Vec<f64>,
    pub constraint_type: ConstraintType,
    pub rhs: f64,
}
