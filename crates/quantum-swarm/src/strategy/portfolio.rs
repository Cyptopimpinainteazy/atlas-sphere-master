//! Portfolio optimization strategy
//!
//! Implements portfolio construction and rebalancing:
//! - Mean-variance optimization (Markowitz)
//! - Risk parity
//! - Maximum Sharpe ratio
//! - Dynamic rebalancing signals

use super::{
    MarketData, OptimizationMethod, PortfolioConfig, Signal, Strategy, StrategyMetrics,
    StrategyType,
};
use crate::error::SwarmResult;
use crate::evolution::Genome;
use crate::types::{RiskProfile, StrategyId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Portfolio allocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioAllocation {
    /// Asset allocations (0-1, sum to 1)
    pub weights: HashMap<String, f64>,
    /// Expected return
    pub expected_return: f64,
    /// Expected volatility
    pub expected_volatility: f64,
    /// Sharpe ratio
    pub sharpe_ratio: f64,
}

impl PortfolioAllocation {
    /// Create equal weight allocation
    pub fn equal_weight(assets: &[String]) -> Self {
        let weight = 1.0 / assets.len() as f64;
        let weights: HashMap<_, _> = assets.iter().map(|a| (a.clone(), weight)).collect();

        Self {
            weights,
            expected_return: 0.0,
            expected_volatility: 0.0,
            sharpe_ratio: 0.0,
        }
    }

    /// Get weight for asset
    pub fn weight(&self, asset: &str) -> f64 {
        self.weights.get(asset).copied().unwrap_or(0.0)
    }

    /// Normalize weights to sum to 1
    pub fn normalize(&mut self) {
        let sum: f64 = self.weights.values().sum();
        if sum > 0.0 {
            for weight in self.weights.values_mut() {
                *weight /= sum;
            }
        }
    }
}

/// Rebalance signal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebalanceSignal {
    /// Asset to rebalance
    pub asset: String,
    /// Current weight
    pub current_weight: f64,
    /// Target weight
    pub target_weight: f64,
    /// Required trade size (positive = buy, negative = sell)
    pub trade_size: f64,
    /// Urgency (0-1)
    pub urgency: f64,
}

/// Portfolio strategy
pub struct PortfolioStrategy {
    /// Strategy ID
    id: StrategyId,
    /// Genome parameters
    genome: Genome,
    /// Configuration
    config: PortfolioConfig,
    /// Strategy type
    strategy_type: StrategyType,
    /// Performance metrics
    metrics: StrategyMetrics,
    /// Current allocation
    current_allocation: PortfolioAllocation,
    /// Target allocation
    target_allocation: PortfolioAllocation,
    /// Risk profile
    risk: RiskProfile,
    /// Historical returns for optimization
    returns_history: HashMap<String, Vec<f64>>,
}

impl PortfolioStrategy {
    /// Create new portfolio strategy
    pub fn new(genome: Genome, config: PortfolioConfig) -> Self {
        let allocation = PortfolioAllocation::equal_weight(&config.assets);

        Self {
            id: genome.id,
            strategy_type: StrategyType::Portfolio(config.clone()),
            genome,
            config,
            metrics: StrategyMetrics::default(),
            current_allocation: allocation.clone(),
            target_allocation: allocation,
            risk: RiskProfile::default(),
            returns_history: HashMap::new(),
        }
    }

    /// Update returns history
    pub fn update_returns(&mut self, asset: &str, return_val: f64) {
        self.returns_history
            .entry(asset.to_string())
            .or_insert_with(Vec::new)
            .push(return_val);

        // Keep last 100 observations
        if let Some(returns) = self.returns_history.get_mut(asset) {
            if returns.len() > 100 {
                returns.remove(0);
            }
        }
    }

    /// Optimize portfolio allocation
    pub fn optimize(&mut self) -> PortfolioAllocation {
        match self.config.optimization {
            OptimizationMethod::MeanVariance => self.mean_variance_optimize(),
            OptimizationMethod::RiskParity => self.risk_parity_optimize(),
            OptimizationMethod::MaxSharpe => self.max_sharpe_optimize(),
            OptimizationMethod::MinVariance => self.min_variance_optimize(),
            OptimizationMethod::BlackLitterman => self.black_litterman_optimize(),
        }
    }

    /// Mean-variance optimization
    fn mean_variance_optimize(&self) -> PortfolioAllocation {
        // Simplified mean-variance: weight by Sharpe-like ratio
        let mut weights = HashMap::new();

        for asset in &self.config.assets {
            let (mean, std) = self.calculate_stats(asset);
            let sharpe = if std > 0.0 { mean / std } else { 0.0 };
            weights.insert(asset.clone(), sharpe.max(0.0));
        }

        // Normalize
        let sum: f64 = weights.values().sum();
        if sum > 0.0 {
            for weight in weights.values_mut() {
                *weight /= sum;
            }
        } else {
            // Equal weight fallback
            let w = 1.0 / self.config.assets.len() as f64;
            for asset in &self.config.assets {
                weights.insert(asset.clone(), w);
            }
        }

        let (exp_ret, exp_vol) = self.calculate_portfolio_stats(&weights);
        let sharpe = if exp_vol > 0.0 {
            exp_ret / exp_vol
        } else {
            0.0
        };

        PortfolioAllocation {
            weights,
            expected_return: exp_ret,
            expected_volatility: exp_vol,
            sharpe_ratio: sharpe,
        }
    }

    /// Risk parity optimization
    fn risk_parity_optimize(&self) -> PortfolioAllocation {
        let mut weights = HashMap::new();
        let mut total_inv_vol = 0.0;

        // Weight by inverse volatility
        for asset in &self.config.assets {
            let (_, std) = self.calculate_stats(asset);
            let inv_vol = if std > 0.0 { 1.0 / std } else { 1.0 };
            weights.insert(asset.clone(), inv_vol);
            total_inv_vol += inv_vol;
        }

        // Normalize
        for weight in weights.values_mut() {
            *weight /= total_inv_vol;
        }

        let (exp_ret, exp_vol) = self.calculate_portfolio_stats(&weights);
        let sharpe = if exp_vol > 0.0 {
            exp_ret / exp_vol
        } else {
            0.0
        };

        PortfolioAllocation {
            weights,
            expected_return: exp_ret,
            expected_volatility: exp_vol,
            sharpe_ratio: sharpe,
        }
    }

    /// Maximum Sharpe ratio optimization
    fn max_sharpe_optimize(&self) -> PortfolioAllocation {
        // Similar to mean-variance for this simplified version
        self.mean_variance_optimize()
    }

    /// Minimum variance optimization  
    fn min_variance_optimize(&self) -> PortfolioAllocation {
        // Weight by inverse variance
        let mut weights = HashMap::new();
        let mut total_inv_var = 0.0;

        for asset in &self.config.assets {
            let (_, std) = self.calculate_stats(asset);
            let inv_var = if std > 0.0 { 1.0 / (std * std) } else { 1.0 };
            weights.insert(asset.clone(), inv_var);
            total_inv_var += inv_var;
        }

        for weight in weights.values_mut() {
            *weight /= total_inv_var;
        }

        let (exp_ret, exp_vol) = self.calculate_portfolio_stats(&weights);
        let sharpe = if exp_vol > 0.0 {
            exp_ret / exp_vol
        } else {
            0.0
        };

        PortfolioAllocation {
            weights,
            expected_return: exp_ret,
            expected_volatility: exp_vol,
            sharpe_ratio: sharpe,
        }
    }

    /// Black-Litterman optimization (simplified)
    fn black_litterman_optimize(&self) -> PortfolioAllocation {
        // Start with market-cap weights (simulated as equal)
        // Apply views from genome parameters
        let mut weights = HashMap::new();
        let base_weight = 1.0 / self.config.assets.len() as f64;

        for (i, asset) in self.config.assets.iter().enumerate() {
            // Get view from genome
            let view_strength = self
                .genome
                .genes
                .get(i)
                .and_then(|g| g.as_float())
                .unwrap_or(0.0);

            // Adjust weight based on view (-0.5 to +0.5)
            let adjusted = base_weight * (1.0 + view_strength);
            weights.insert(asset.clone(), adjusted.max(0.0));
        }

        // Normalize
        let sum: f64 = weights.values().sum();
        if sum > 0.0 {
            for weight in weights.values_mut() {
                *weight /= sum;
            }
        }

        let (exp_ret, exp_vol) = self.calculate_portfolio_stats(&weights);
        let sharpe = if exp_vol > 0.0 {
            exp_ret / exp_vol
        } else {
            0.0
        };

        PortfolioAllocation {
            weights,
            expected_return: exp_ret,
            expected_volatility: exp_vol,
            sharpe_ratio: sharpe,
        }
    }

    /// Calculate mean and std for asset
    fn calculate_stats(&self, asset: &str) -> (f64, f64) {
        let returns = self.returns_history.get(asset);

        match returns {
            Some(r) if !r.is_empty() => {
                let n = r.len() as f64;
                let mean = r.iter().sum::<f64>() / n;
                let variance = r.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
                (mean, variance.sqrt())
            }
            _ => (0.0, 0.1), // Default
        }
    }

    /// Calculate portfolio expected return and volatility
    fn calculate_portfolio_stats(&self, weights: &HashMap<String, f64>) -> (f64, f64) {
        let mut exp_return = 0.0;
        let mut exp_variance = 0.0;

        for (asset, weight) in weights {
            let (mean, std) = self.calculate_stats(asset);
            exp_return += weight * mean;
            exp_variance += weight * weight * std * std;
        }

        (exp_return, exp_variance.sqrt())
    }

    /// Generate rebalance signals
    pub fn check_rebalance(&self, current_values: &HashMap<String, f64>) -> Vec<RebalanceSignal> {
        let mut signals = Vec::new();

        // Calculate current weights
        let total_value: f64 = current_values.values().sum();
        if total_value <= 0.0 {
            return signals;
        }

        for asset in &self.config.assets {
            let current_value = current_values.get(asset).copied().unwrap_or(0.0);
            let current_weight = current_value / total_value;
            let target_weight = self.target_allocation.weight(asset);

            let drift = (current_weight - target_weight).abs();

            if drift > self.config.rebalance_threshold {
                let trade_size = (target_weight - current_weight) * total_value;
                let urgency = (drift / self.config.rebalance_threshold).min(1.0);

                signals.push(RebalanceSignal {
                    asset: asset.clone(),
                    current_weight,
                    target_weight,
                    trade_size,
                    urgency,
                });
            }
        }

        signals
    }

    /// Update current allocation
    pub fn update_allocation(&mut self, allocation: PortfolioAllocation) {
        self.current_allocation = allocation;
    }

    /// Set target allocation
    pub fn set_target(&mut self, allocation: PortfolioAllocation) {
        self.target_allocation = allocation;
    }
}

impl Strategy for PortfolioStrategy {
    fn id(&self) -> StrategyId {
        self.id
    }

    fn strategy_type(&self) -> &StrategyType {
        &self.strategy_type
    }

    fn genome(&self) -> &Genome {
        &self.genome
    }

    fn evaluate(&self, market_data: &MarketData) -> SwarmResult<Vec<Signal>> {
        let mut signals = Vec::new();

        // Check for rebalance needs
        let rebalance_signals = self.check_rebalance(&market_data.prices);

        for rebal in rebalance_signals {
            if rebal.trade_size > 0.0 {
                signals.push(Signal::buy(&rebal.asset, rebal.urgency));
            } else {
                signals.push(Signal::sell(&rebal.asset, rebal.urgency));
            }
        }

        Ok(signals)
    }

    fn risk_profile(&self) -> RiskProfile {
        self.risk.clone()
    }

    fn metrics(&self) -> StrategyMetrics {
        self.metrics.clone()
    }

    fn update_from_genome(&mut self, genome: &Genome) {
        self.genome = genome.clone();
        self.id = genome.id;

        // Re-optimize with new genome
        self.target_allocation = self.optimize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_equal_weight_allocation() {
        let assets = vec!["BTC".to_string(), "ETH".to_string(), "SOL".to_string()];
        let allocation = PortfolioAllocation::equal_weight(&assets);

        assert_eq!(allocation.weights.len(), 3);
        assert!((allocation.weight("BTC") - 1.0 / 3.0).abs() < 0.001);
    }

    #[test]
    fn test_portfolio_strategy() {
        let genome = Genome::new_float(3, -0.5, 0.5);
        let config = PortfolioConfig {
            assets: vec!["BTC".to_string(), "ETH".to_string(), "SOL".to_string()],
            rebalance_threshold: 0.05,
            risk_tolerance: 0.5,
            optimization: OptimizationMethod::RiskParity,
        };

        let mut strategy = PortfolioStrategy::new(genome, config);

        // Add some returns
        strategy.update_returns("BTC", 0.02);
        strategy.update_returns("BTC", 0.01);
        strategy.update_returns("ETH", 0.03);
        strategy.update_returns("ETH", -0.01);
        strategy.update_returns("SOL", 0.05);
        strategy.update_returns("SOL", -0.02);

        let allocation = strategy.optimize();

        // Should sum to 1
        let sum: f64 = allocation.weights.values().sum();
        assert!((sum - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_rebalance_signals() {
        let genome = Genome::new_float(2, -0.5, 0.5);
        let config = PortfolioConfig {
            assets: vec!["BTC".to_string(), "ETH".to_string()],
            rebalance_threshold: 0.05,
            risk_tolerance: 0.5,
            optimization: OptimizationMethod::MeanVariance,
        };

        let mut strategy = PortfolioStrategy::new(genome, config);
        strategy.target_allocation =
            PortfolioAllocation::equal_weight(&["BTC".to_string(), "ETH".to_string()]);

        // Current: 60% BTC, 40% ETH (10% drift)
        let current_values: HashMap<_, _> =
            vec![("BTC".to_string(), 6000.0), ("ETH".to_string(), 4000.0)]
                .into_iter()
                .collect();

        let signals = strategy.check_rebalance(&current_values);

        // Should generate rebalance signals
        assert!(!signals.is_empty());
    }
}
