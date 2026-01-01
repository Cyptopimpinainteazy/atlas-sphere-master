//! Fitness scoring for strategy evaluation

use crate::chromosome::Chromosome;
use crate::error::Result;
use serde::{Deserialize, Serialize};

/// Fitness score with multiple components
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FitnessScore {
    /// Total profit/loss
    pub pnl: f64,
    /// Sharpe ratio
    pub sharpe_ratio: f64,
    /// Maximum drawdown (negative)
    pub max_drawdown: f64,
    /// Win rate (0.0 - 1.0)
    pub win_rate: f64,
    /// Number of trades
    pub trade_count: usize,
    /// Average trade duration
    pub avg_duration: f64,
    /// Profit factor (gross profit / gross loss)
    pub profit_factor: f64,
    /// Sortino ratio
    pub sortino_ratio: f64,
    /// Calmar ratio
    pub calmar_ratio: f64,
    /// Average trade PnL
    pub avg_trade: f64,
    /// Volatility (annualized)
    pub volatility: f64,
    /// Custom score component
    pub custom: f64,
}

impl FitnessScore {
    /// Calculate total fitness score (weighted combination)
    pub fn total_score(&self) -> f64 {
        // Weights for different components
        const W_PNL: f64 = 0.25;
        const W_SHARPE: f64 = 0.20;
        const W_DRAWDOWN: f64 = 0.15;
        const W_WIN_RATE: f64 = 0.10;
        const W_PROFIT_FACTOR: f64 = 0.15;
        const W_SORTINO: f64 = 0.10;
        const W_CALMAR: f64 = 0.05;

        // Normalize components to similar scales
        let pnl_norm = self.pnl.tanh(); // Squash to [-1, 1]
        let sharpe_norm = (self.sharpe_ratio / 3.0).tanh(); // 3.0 is considered excellent
        let drawdown_norm = 1.0 + self.max_drawdown.max(-1.0); // Convert to [0, 1]
        let win_rate_norm = self.win_rate;
        let pf_norm = (self.profit_factor / 2.0).min(1.0); // Cap at 2.0
        let sortino_norm = (self.sortino_ratio / 4.0).tanh();
        let calmar_norm = (self.calmar_ratio / 3.0).tanh();

        W_PNL * pnl_norm
            + W_SHARPE * sharpe_norm
            + W_DRAWDOWN * drawdown_norm
            + W_WIN_RATE * win_rate_norm
            + W_PROFIT_FACTOR * pf_norm
            + W_SORTINO * sortino_norm
            + W_CALMAR * calmar_norm
            + self.custom * 0.0 // Custom weight can be added
    }

    /// Create a score representing complete failure
    pub fn failure() -> Self {
        Self {
            pnl: f64::NEG_INFINITY,
            sharpe_ratio: f64::NEG_INFINITY,
            max_drawdown: -1.0,
            win_rate: 0.0,
            trade_count: 0,
            avg_duration: 0.0,
            profit_factor: 0.0,
            sortino_ratio: f64::NEG_INFINITY,
            calmar_ratio: f64::NEG_INFINITY,
            avg_trade: 0.0,
            volatility: 0.0,
            custom: 0.0,
        }
    }

    /// Check if this is a valid score
    pub fn is_valid(&self) -> bool {
        !self.pnl.is_nan() && !self.pnl.is_infinite()
    }
}

/// Trait for fitness evaluation
pub trait FitnessEvaluator: Send + Sync {
    /// Evaluate fitness of a chromosome
    fn evaluate(&self, chromosome: &Chromosome) -> Result<FitnessScore>;

    /// Get evaluator name
    fn name(&self) -> &'static str;
}

/// PnL-based fitness evaluator using historical simulation
pub struct PnLFitness {
    /// Historical price data for simulation
    price_data: Vec<PricePoint>,
    /// Initial capital
    initial_capital: f64,
    /// Risk-free rate for Sharpe calculation
    risk_free_rate: f64,
    /// Number of simulation runs
    num_simulations: usize,
}

/// A single price point
#[derive(Debug, Clone)]
pub struct PricePoint {
    pub timestamp: u64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

impl PnLFitness {
    pub fn new(price_data: Vec<PricePoint>, initial_capital: f64) -> Self {
        Self {
            price_data,
            initial_capital,
            risk_free_rate: 0.02, // 2% annual
            num_simulations: 1,
        }
    }

    pub fn with_risk_free_rate(mut self, rate: f64) -> Self {
        self.risk_free_rate = rate;
        self
    }

    pub fn with_simulations(mut self, n: usize) -> Self {
        self.num_simulations = n;
        self
    }

    /// Simulate strategy execution
    fn simulate(&self, _chromosome: &Chromosome) -> SimulationResult {
        // In a real implementation, this would:
        // 1. Load strategy bytecode into X3 VM
        // 2. Feed price data through the strategy
        // 3. Track trades, PnL, equity curve
        // 4. Calculate all metrics

        // For now, return placeholder results
        // This would be replaced with actual X3 VM integration
        SimulationResult {
            trades: vec![],
            equity_curve: vec![self.initial_capital],
            final_capital: self.initial_capital,
        }
    }

    /// Calculate metrics from simulation result
    fn calculate_metrics(&self, result: &SimulationResult) -> FitnessScore {
        let pnl = result.final_capital - self.initial_capital;
        let pnl_percent = pnl / self.initial_capital;

        // Calculate returns
        let returns: Vec<f64> = result
            .equity_curve
            .windows(2)
            .map(|w| (w[1] - w[0]) / w[0])
            .collect();

        if returns.is_empty() {
            return FitnessScore::failure();
        }

        // Average return
        let avg_return = returns.iter().sum::<f64>() / returns.len() as f64;

        // Standard deviation
        let variance = returns
            .iter()
            .map(|r| (r - avg_return).powi(2))
            .sum::<f64>()
            / returns.len() as f64;
        let std_dev = variance.sqrt();

        // Sharpe ratio (annualized)
        let sharpe = if std_dev > 0.0 {
            (avg_return - self.risk_free_rate / 252.0) / std_dev * (252.0_f64).sqrt()
        } else {
            0.0
        };

        // Maximum drawdown
        let mut peak = result.equity_curve[0];
        let mut max_dd = 0.0_f64;
        for &equity in &result.equity_curve {
            if equity > peak {
                peak = equity;
            }
            let dd = (peak - equity) / peak;
            if dd > max_dd {
                max_dd = dd;
            }
        }

        // Downside deviation (for Sortino)
        let downside_returns: Vec<f64> = returns.iter().filter(|&&r| r < 0.0).copied().collect();

        let downside_dev = if !downside_returns.is_empty() {
            (downside_returns.iter().map(|r| r.powi(2)).sum::<f64>()
                / downside_returns.len() as f64)
                .sqrt()
        } else {
            0.001 // Small value to avoid division by zero
        };

        // Sortino ratio
        let sortino = if downside_dev > 0.0 {
            (avg_return - self.risk_free_rate / 252.0) / downside_dev * (252.0_f64).sqrt()
        } else {
            sharpe // Fall back to Sharpe if no downside
        };

        // Calmar ratio
        let calmar = if max_dd > 0.0 {
            pnl_percent / max_dd
        } else {
            pnl_percent * 10.0 // Bonus for no drawdown
        };

        // Trade statistics
        let winning_trades = result.trades.iter().filter(|t| t.pnl > 0.0).count();
        let total_trades = result.trades.len();
        let win_rate = if total_trades > 0 {
            winning_trades as f64 / total_trades as f64
        } else {
            0.0
        };

        // Profit factor
        let gross_profit: f64 = result
            .trades
            .iter()
            .filter(|t| t.pnl > 0.0)
            .map(|t| t.pnl)
            .sum();
        let gross_loss: f64 = result
            .trades
            .iter()
            .filter(|t| t.pnl < 0.0)
            .map(|t| t.pnl.abs())
            .sum();
        let profit_factor = if gross_loss > 0.0 {
            gross_profit / gross_loss
        } else if gross_profit > 0.0 {
            10.0 // Cap at 10
        } else {
            0.0
        };

        // Average trade duration
        let avg_duration = if total_trades > 0 {
            result.trades.iter().map(|t| t.duration as f64).sum::<f64>() / total_trades as f64
        } else {
            0.0
        };

        FitnessScore {
            pnl: pnl_percent,
            sharpe_ratio: sharpe,
            max_drawdown: -max_dd,
            win_rate,
            trade_count: total_trades,
            avg_duration,
            profit_factor,
            sortino_ratio: sortino,
            calmar_ratio: calmar,
            avg_trade: 0.0,
            volatility: 0.0,
            custom: 0.0,
        }
    }
}

impl FitnessEvaluator for PnLFitness {
    fn evaluate(&self, chromosome: &Chromosome) -> Result<FitnessScore> {
        let mut total_score = FitnessScore::default();

        for _ in 0..self.num_simulations {
            let result = self.simulate(chromosome);
            let score = self.calculate_metrics(&result);

            // Aggregate scores
            total_score.pnl += score.pnl;
            total_score.sharpe_ratio += score.sharpe_ratio;
            total_score.max_drawdown += score.max_drawdown;
            total_score.win_rate += score.win_rate;
            total_score.trade_count += score.trade_count;
            total_score.profit_factor += score.profit_factor;
            total_score.sortino_ratio += score.sortino_ratio;
            total_score.calmar_ratio += score.calmar_ratio;
        }

        // Average
        let n = self.num_simulations as f64;
        total_score.pnl /= n;
        total_score.sharpe_ratio /= n;
        total_score.max_drawdown /= n;
        total_score.win_rate /= n;
        total_score.profit_factor /= n;
        total_score.sortino_ratio /= n;
        total_score.calmar_ratio /= n;

        Ok(total_score)
    }

    fn name(&self) -> &'static str {
        "PnLFitness"
    }
}

/// Result of a strategy simulation
#[derive(Debug, Clone)]
struct SimulationResult {
    trades: Vec<Trade>,
    equity_curve: Vec<f64>,
    final_capital: f64,
}

/// A single trade
#[derive(Debug, Clone)]
struct Trade {
    pnl: f64,
    duration: u64,
}

/// Multi-objective fitness evaluator using Pareto ranking
pub struct ParetoFitness {
    evaluators: Vec<Box<dyn FitnessEvaluator>>,
}

impl ParetoFitness {
    pub fn new() -> Self {
        Self {
            evaluators: Vec::new(),
        }
    }

    pub fn add<E: FitnessEvaluator + 'static>(mut self, evaluator: E) -> Self {
        self.evaluators.push(Box::new(evaluator));
        self
    }
}

impl Default for ParetoFitness {
    fn default() -> Self {
        Self::new()
    }
}

impl FitnessEvaluator for ParetoFitness {
    fn evaluate(&self, chromosome: &Chromosome) -> Result<FitnessScore> {
        let mut combined = FitnessScore::default();

        for evaluator in &self.evaluators {
            let score = evaluator.evaluate(chromosome)?;
            // Simple aggregation - could be enhanced with Pareto ranking
            combined.pnl += score.pnl;
            combined.sharpe_ratio += score.sharpe_ratio;
            combined.max_drawdown = combined.max_drawdown.min(score.max_drawdown);
            combined.win_rate += score.win_rate;
            combined.profit_factor += score.profit_factor;
        }

        let n = self.evaluators.len() as f64;
        if n > 0.0 {
            combined.pnl /= n;
            combined.sharpe_ratio /= n;
            combined.win_rate /= n;
            combined.profit_factor /= n;
        }

        Ok(combined)
    }

    fn name(&self) -> &'static str {
        "ParetoFitness"
    }
}

/// Mock fitness evaluator for testing
pub struct MockFitness {
    base_score: f64,
    variance: f64,
}

impl MockFitness {
    pub fn new(base_score: f64, variance: f64) -> Self {
        Self {
            base_score,
            variance,
        }
    }
}

impl FitnessEvaluator for MockFitness {
    fn evaluate(&self, chromosome: &Chromosome) -> Result<FitnessScore> {
        // Generate deterministic score based on chromosome hash
        let hash = chromosome.hash();
        let hash_value = u64::from_le_bytes([
            hash[0], hash[1], hash[2], hash[3], hash[4], hash[5], hash[6], hash[7],
        ]);

        let normalized = (hash_value as f64) / (u64::MAX as f64);
        let score = self.base_score + (normalized - 0.5) * 2.0 * self.variance;

        Ok(FitnessScore {
            pnl: score,
            sharpe_ratio: score * 2.0,
            max_drawdown: -0.1 * (1.0 - normalized),
            win_rate: 0.5 + normalized * 0.3,
            trade_count: 100,
            avg_duration: 60.0,
            profit_factor: 1.0 + score,
            sortino_ratio: score * 2.5,
            calmar_ratio: score * 3.0,
            avg_trade: score * 0.01,
            volatility: 0.15,
            custom: 0.0,
        })
    }

    fn name(&self) -> &'static str {
        "MockFitness"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fitness_score_total() {
        let score = FitnessScore {
            pnl: 0.5,
            sharpe_ratio: 2.0,
            max_drawdown: -0.1,
            win_rate: 0.6,
            trade_count: 100,
            avg_duration: 60.0,
            profit_factor: 1.5,
            sortino_ratio: 2.5,
            calmar_ratio: 3.0,
            avg_trade: 0.05,
            volatility: 0.15,
            custom: 0.0,
        };

        let total = score.total_score();
        assert!(total > 0.0);
        assert!(total < 1.0);
    }

    #[test]
    fn test_mock_fitness() {
        let bytecode = vec![0x20, 0x64, 0x00, 0x00];
        let chromosome = Chromosome::from_bytecode(bytecode).unwrap();

        let evaluator = MockFitness::new(0.5, 0.2);
        let score = evaluator.evaluate(&chromosome).unwrap();

        assert!(score.is_valid());
        assert!(score.pnl > 0.0);
    }

    #[test]
    fn test_fitness_score_failure() {
        let score = FitnessScore::failure();
        assert!(!score.is_valid());
    }
}
