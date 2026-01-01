//! X3 Strategy Simulation Job
//!
//! Executes X3 bytecode strategies against historical or live market data,
//! scoring them based on PnL, Sharpe ratio, drawdown, and other metrics.

use crate::error::{SwarmError, SwarmResult};
use crate::jobs::{JobOutput, JobType, SwarmJob};
use crate::task::TaskPriority;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Configuration for X3 simulation job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimConfig {
    /// Number of strategies to evaluate
    pub population_size: usize,
    /// Generations for evolution
    pub generations: usize,
    /// Historical data window (seconds)
    pub data_window_secs: u64,
    /// Simulation timestep (ms)
    pub timestep_ms: u64,
    /// Initial capital for each strategy
    pub initial_capital: f64,
    /// Trading fee rate
    pub fee_rate: f64,
    /// Slippage rate
    pub slippage_rate: f64,
    /// Maximum position size
    pub max_position_pct: f64,
    /// Mutation rate for evolution
    pub mutation_rate: f64,
    /// Crossover rate for evolution
    pub crossover_rate: f64,
    /// Elite preservation ratio
    pub elite_ratio: f64,
    /// Target assets to trade
    pub assets: Vec<String>,
    /// Random seed for reproducibility
    pub seed: Option<u64>,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            population_size: 100,
            generations: 50,
            data_window_secs: 86400, // 24 hours
            timestep_ms: 1000,       // 1 second
            initial_capital: 100_000.0,
            fee_rate: 0.001,
            slippage_rate: 0.0005,
            max_position_pct: 0.25,
            mutation_rate: 0.1,
            crossover_rate: 0.7,
            elite_ratio: 0.1,
            assets: vec!["ETH/USDC".to_string(), "BTC/USDC".to_string()],
            seed: None,
        }
    }
}

/// A strategy candidate for evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyCandidate {
    /// Strategy ID (hash of bytecode)
    pub id: [u8; 32],
    /// X3 bytecode
    pub bytecode: Vec<u8>,
    /// Generation created
    pub generation: usize,
    /// Parent strategy IDs (for lineage)
    pub parents: Vec<[u8; 32]>,
    /// Fitness score
    pub fitness: Option<FitnessMetrics>,
}

/// Fitness metrics for strategy evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitnessMetrics {
    /// Total PnL percentage
    pub pnl_pct: f64,
    /// Sharpe ratio (risk-adjusted return)
    pub sharpe_ratio: f64,
    /// Sortino ratio (downside risk-adjusted)
    pub sortino_ratio: f64,
    /// Maximum drawdown percentage
    pub max_drawdown_pct: f64,
    /// Win rate (profitable trades / total trades)
    pub win_rate: f64,
    /// Profit factor (gross profit / gross loss)
    pub profit_factor: f64,
    /// Calmar ratio (return / max drawdown)
    pub calmar_ratio: f64,
    /// Total number of trades
    pub trade_count: usize,
    /// Average trade duration (seconds)
    pub avg_trade_duration: f64,
    /// Gas cost estimate
    pub gas_estimate: u64,
    /// Execution latency (ms)
    pub latency_ms: f64,
    /// Composite score (weighted combination)
    pub total_score: f64,
}

impl FitnessMetrics {
    /// Calculate composite score from individual metrics
    pub fn calculate_total(&mut self) {
        const W_PNL: f64 = 0.25;
        const W_SHARPE: f64 = 0.20;
        const W_DRAWDOWN: f64 = 0.15;
        const W_WIN_RATE: f64 = 0.10;
        const W_PROFIT_FACTOR: f64 = 0.15;
        const W_SORTINO: f64 = 0.10;
        const W_LATENCY: f64 = 0.05;

        let pnl_norm = self.pnl_pct.tanh();
        let sharpe_norm = (self.sharpe_ratio / 3.0).tanh();
        let drawdown_norm = 1.0 + self.max_drawdown_pct.max(-1.0);
        let pf_norm = (self.profit_factor / 2.0).min(1.0);
        let sortino_norm = (self.sortino_ratio / 4.0).tanh();
        let latency_norm = 1.0 - (self.latency_ms / 1000.0).min(1.0);

        self.total_score = W_PNL * pnl_norm
            + W_SHARPE * sharpe_norm
            + W_DRAWDOWN * drawdown_norm
            + W_WIN_RATE * self.win_rate
            + W_PROFIT_FACTOR * pf_norm
            + W_SORTINO * sortino_norm
            + W_LATENCY * latency_norm;
    }
}

/// Result from X3 simulation job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    /// Top performing strategies (sorted by fitness)
    pub top_strategies: Vec<StrategyCandidate>,
    /// Generation statistics
    pub generation_stats: Vec<GenerationStats>,
    /// Best fitness achieved
    pub best_fitness: f64,
    /// Total strategies evaluated
    pub total_evaluated: usize,
    /// Execution duration (ms)
    pub duration_ms: u64,
    /// Compute units consumed
    pub compute_units: u64,
    /// Result hash for verification
    pub result_hash: [u8; 32],
}

/// Statistics for a single generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationStats {
    pub generation: usize,
    pub best_fitness: f64,
    pub avg_fitness: f64,
    pub worst_fitness: f64,
    pub diversity: f64,
}

/// X3 Strategy Simulation Job
pub struct X3SimulationJob {
    /// Job configuration
    pub config: SimConfig,
    /// Seed strategies (bytecode)
    pub seed_strategies: Vec<Vec<u8>>,
    /// Historical market data
    pub market_data: Vec<MarketDataPoint>,
}

/// Market data point for simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketDataPoint {
    pub timestamp: u64,
    pub asset: String,
    pub price: f64,
    pub volume: f64,
    pub bid: f64,
    pub ask: f64,
}

impl X3SimulationJob {
    pub fn new(config: SimConfig) -> Self {
        Self {
            config,
            seed_strategies: Vec::new(),
            market_data: Vec::new(),
        }
    }

    /// Add seed strategies
    pub fn with_seeds(mut self, seeds: Vec<Vec<u8>>) -> Self {
        self.seed_strategies = seeds;
        self
    }

    /// Add market data
    pub fn with_market_data(mut self, data: Vec<MarketDataPoint>) -> Self {
        self.market_data = data;
        self
    }

    /// Execute simulation using x3-evolution
    fn run_simulation(&self) -> SwarmResult<SimulationResult> {
        use blake3::hash;
        use std::time::Instant;

        let start = Instant::now();

        // In production, this would call into x3-evolution::EvolutionEngine
        // For now, generate mock results
        let mut top_strategies = Vec::new();
        let mut generation_stats = Vec::new();
        let mut best_fitness = 0.0;

        // Simulate evolution generations
        for gen in 0..self.config.generations.min(10) {
            let gen_best = 0.5 + (gen as f64 * 0.05);
            let gen_avg = 0.3 + (gen as f64 * 0.03);

            generation_stats.push(GenerationStats {
                generation: gen,
                best_fitness: gen_best,
                avg_fitness: gen_avg,
                worst_fitness: 0.1,
                diversity: 0.8 - (gen as f64 * 0.02),
            });

            if gen_best > best_fitness {
                best_fitness = gen_best;
            }
        }

        // Generate top strategies
        for i in 0..5 {
            let bytecode = vec![0x20, 0x64 + i as u8, 0x00, 0x00, 0x30, 0x01, 0x00, 0x00];
            let id = hash(&bytecode).into();

            let mut fitness = FitnessMetrics {
                pnl_pct: 15.0 - (i as f64 * 2.0),
                sharpe_ratio: 2.5 - (i as f64 * 0.3),
                sortino_ratio: 3.0 - (i as f64 * 0.4),
                max_drawdown_pct: -5.0 - (i as f64),
                win_rate: 0.65 - (i as f64 * 0.02),
                profit_factor: 1.8 - (i as f64 * 0.1),
                calmar_ratio: 3.0 - (i as f64 * 0.5),
                trade_count: 150 - (i * 10),
                avg_trade_duration: 3600.0,
                gas_estimate: 50000 + (i as u64 * 5000),
                latency_ms: 10.0 + (i as f64 * 2.0),
                total_score: 0.0,
            };
            fitness.calculate_total();

            top_strategies.push(StrategyCandidate {
                id,
                bytecode,
                generation: self.config.generations,
                parents: Vec::new(),
                fitness: Some(fitness),
            });
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        let compute_units =
            self.config.population_size as u64 * self.config.generations as u64 * 100; // 100 CU per strategy-generation

        // Calculate result hash
        let mut hasher = blake3::Hasher::new();
        for strat in &top_strategies {
            hasher.update(&strat.id);
        }
        let result_hash: [u8; 32] = hasher.finalize().into();

        Ok(SimulationResult {
            top_strategies,
            generation_stats,
            best_fitness,
            total_evaluated: self.config.population_size * self.config.generations,
            duration_ms,
            compute_units,
            result_hash,
        })
    }
}

impl SwarmJob for X3SimulationJob {
    fn job_type(&self) -> JobType {
        JobType::X3Simulation
    }

    fn compute_units(&self) -> u64 {
        // Estimate: 100 CU per strategy per generation
        (self.config.population_size * self.config.generations * 100) as u64
    }

    fn timeout(&self) -> Duration {
        // 1 second per generation, minimum 60 seconds
        Duration::from_secs((self.config.generations as u64).max(60))
    }

    fn execute(&self) -> SwarmResult<JobOutput> {
        let result = self.run_simulation()?;
        Ok(JobOutput::X3Simulation(result))
    }

    fn verify(&self, result: &JobOutput) -> SwarmResult<bool> {
        match result {
            JobOutput::X3Simulation(sim_result) => {
                // Verify result hash
                let mut hasher = blake3::Hasher::new();
                for strat in &sim_result.top_strategies {
                    hasher.update(&strat.id);
                }
                let expected_hash: [u8; 32] = hasher.finalize().into();

                Ok(expected_hash == sim_result.result_hash)
            }
            _ => Err(SwarmError::InvalidResult("Wrong result type".into())),
        }
    }

    fn priority(&self) -> TaskPriority {
        TaskPriority::High
    }

    fn requires_gpu(&self) -> bool {
        true
    }

    fn min_vram_mb(&self) -> u32 {
        1024 // 1GB for large populations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sim_config_default() {
        let config = SimConfig::default();
        assert_eq!(config.population_size, 100);
        assert_eq!(config.generations, 50);
    }

    #[test]
    fn test_fitness_calculation() {
        let mut fitness = FitnessMetrics {
            pnl_pct: 10.0,
            sharpe_ratio: 2.0,
            sortino_ratio: 2.5,
            max_drawdown_pct: -5.0,
            win_rate: 0.6,
            profit_factor: 1.5,
            calmar_ratio: 2.0,
            trade_count: 100,
            avg_trade_duration: 3600.0,
            gas_estimate: 50000,
            latency_ms: 15.0,
            total_score: 0.0,
        };

        fitness.calculate_total();
        assert!(fitness.total_score > 0.0);
        assert!(fitness.total_score < 1.0);
    }

    #[test]
    fn test_simulation_job_execution() {
        let config = SimConfig {
            population_size: 10,
            generations: 5,
            ..Default::default()
        };

        let job = X3SimulationJob::new(config);
        let result = job.execute().unwrap();

        if let JobOutput::X3Simulation(sim_result) = result {
            assert!(!sim_result.top_strategies.is_empty());
            assert!(sim_result.best_fitness > 0.0);
        } else {
            panic!("Wrong result type");
        }
    }
}
