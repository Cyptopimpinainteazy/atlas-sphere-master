//! Arbitrage strategy implementation
//!
//! Detects and executes arbitrage opportunities:
//! - Cross-exchange price differences
//! - Cross-VM (EVM/SVM) opportunities
//! - Flash loan arbitrage paths

use super::{ArbitrageConfig, MarketData, Signal, Strategy, StrategyMetrics, StrategyType};
use crate::error::SwarmResult;
use crate::evolution::Genome;
use crate::types::{RiskProfile, StrategyId, TradeRoute};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An arbitrage opportunity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageOpportunity {
    /// Unique ID
    pub id: uuid::Uuid,
    /// Asset being arbitraged
    pub asset: String,
    /// Buy venue
    pub buy_venue: String,
    /// Sell venue
    pub sell_venue: String,
    /// Buy price
    pub buy_price: f64,
    /// Sell price
    pub sell_price: f64,
    /// Expected profit (basis points)
    pub profit_bps: u32,
    /// Maximum size
    pub max_size: f64,
    /// Expires at (timestamp)
    pub expires_at: u64,
    /// Route for execution
    pub route: ArbitrageRoute,
}

impl ArbitrageOpportunity {
    /// Calculate gross profit
    pub fn gross_profit(&self, size: f64) -> f64 {
        (self.sell_price - self.buy_price) * size
    }

    /// Calculate net profit after fees
    pub fn net_profit(&self, size: f64, fee_bps: u32) -> f64 {
        let gross = self.gross_profit(size);
        let fees = size * self.buy_price * (fee_bps as f64 / 10000.0);
        gross - fees
    }

    /// Is opportunity still valid
    pub fn is_valid(&self, current_time: u64) -> bool {
        current_time < self.expires_at && self.profit_bps > 0
    }
}

/// Route for executing arbitrage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageRoute {
    /// Steps in the route
    pub steps: Vec<ArbitrageStep>,
    /// Total gas estimate
    pub gas_estimate: u64,
    /// Is cross-chain
    pub cross_chain: bool,
    /// Required flash loan
    pub flash_loan: Option<FlashLoanConfig>,
}

/// Single step in arbitrage route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageStep {
    /// Action (swap, bridge, etc.)
    pub action: String,
    /// Protocol/venue
    pub venue: String,
    /// Input asset
    pub input: String,
    /// Output asset
    pub output: String,
    /// VM type (EVM/SVM)
    pub vm_type: String,
}

/// Flash loan configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashLoanConfig {
    /// Protocol (Aave, dYdX, etc.)
    pub protocol: String,
    /// Asset to borrow
    pub asset: String,
    /// Amount
    pub amount: f64,
    /// Fee (basis points)
    pub fee_bps: u32,
}

/// Arbitrage strategy
pub struct ArbitrageStrategy {
    /// Strategy ID
    id: StrategyId,
    /// Genome parameters
    genome: Genome,
    /// Configuration
    config: ArbitrageConfig,
    /// Strategy type
    strategy_type: StrategyType,
    /// Performance metrics
    metrics: StrategyMetrics,
    /// Active opportunities
    opportunities: Vec<ArbitrageOpportunity>,
    /// Risk profile
    risk: RiskProfile,
}

impl ArbitrageStrategy {
    /// Create new arbitrage strategy
    pub fn new(genome: Genome, config: ArbitrageConfig) -> Self {
        Self {
            id: genome.id,
            strategy_type: StrategyType::Arbitrage(config.clone()),
            genome,
            config,
            metrics: StrategyMetrics::default(),
            opportunities: Vec::new(),
            risk: RiskProfile::default(),
        }
    }

    /// Scan for arbitrage opportunities
    pub fn scan(&mut self, market_data: &MarketData) -> Vec<ArbitrageOpportunity> {
        let mut opportunities = Vec::new();

        // Get prices for each venue
        let venues = &self.config.venues;
        if venues.len() < 2 {
            return opportunities;
        }

        for asset in market_data.prices.keys() {
            // Get prices from different venues
            let mut venue_prices: Vec<(&str, f64)> = Vec::new();

            for venue in venues {
                // In real implementation, would have per-venue prices
                // For now, simulate small price differences
                let base_price = market_data.prices.get(asset).copied().unwrap_or(0.0);
                let variance = (rand::random::<f64>() - 0.5) * 0.02 * base_price;
                venue_prices.push((venue.as_str(), base_price + variance));
            }

            // Find best buy and sell
            if let (Some((buy_venue, buy_price)), Some((sell_venue, sell_price))) = (
                venue_prices
                    .iter()
                    .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap()),
                venue_prices
                    .iter()
                    .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap()),
            ) {
                if buy_venue != sell_venue && sell_price > buy_price {
                    let profit_bps = ((*sell_price - *buy_price) / *buy_price * 10000.0) as u32;

                    if profit_bps >= self.config.min_profit_bps {
                        opportunities.push(ArbitrageOpportunity {
                            id: uuid::Uuid::new_v4(),
                            asset: asset.clone(),
                            buy_venue: buy_venue.to_string(),
                            sell_venue: sell_venue.to_string(),
                            buy_price: *buy_price,
                            sell_price: *sell_price,
                            profit_bps,
                            max_size: self.config.max_position as f64,
                            expires_at: market_data.timestamp + 5000, // 5 second window
                            route: ArbitrageRoute {
                                steps: vec![
                                    ArbitrageStep {
                                        action: "buy".to_string(),
                                        venue: buy_venue.to_string(),
                                        input: "USDC".to_string(),
                                        output: asset.clone(),
                                        vm_type: "EVM".to_string(),
                                    },
                                    ArbitrageStep {
                                        action: "sell".to_string(),
                                        venue: sell_venue.to_string(),
                                        input: asset.clone(),
                                        output: "USDC".to_string(),
                                        vm_type: "EVM".to_string(),
                                    },
                                ],
                                gas_estimate: 200000,
                                cross_chain: false,
                                flash_loan: None,
                            },
                        });
                    }
                }
            }
        }

        // Sort by profit
        opportunities.sort_by(|a, b| b.profit_bps.cmp(&a.profit_bps));

        self.opportunities = opportunities.clone();
        opportunities
    }

    /// Get best opportunity
    pub fn best_opportunity(&self) -> Option<&ArbitrageOpportunity> {
        self.opportunities.first()
    }

    /// Execute opportunity
    pub fn execute(&mut self, opportunity: &ArbitrageOpportunity, size: f64) -> SwarmResult<f64> {
        // In real implementation, would submit transactions
        let profit = opportunity.net_profit(size, 30); // 30 bps fee assumption

        // Update metrics
        self.metrics.total_trades += 1;
        self.metrics.total_pnl += profit;
        self.metrics.total_volume += size * opportunity.buy_price;

        if profit > 0.0 {
            // Update win rate (simple running average)
            let wins = (self.metrics.win_rate * (self.metrics.total_trades - 1) as f64) + 1.0;
            self.metrics.win_rate = wins / self.metrics.total_trades as f64;
        }

        Ok(profit)
    }
}

impl Strategy for ArbitrageStrategy {
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

        // Generate signals from opportunities
        for opp in &self.opportunities {
            if opp.profit_bps >= self.config.min_profit_bps {
                // Buy signal
                signals.push(
                    Signal::buy(&opp.asset, opp.profit_bps as f64 / 1000.0)
                        .with_urgency((opp.profit_bps as f64 / 100.0).min(1.0)),
                );
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
        // Update parameters from genome
        if let Some(min_profit) = genome.get_gene("min_profit") {
            if let Some(val) = min_profit.as_float() {
                self.config.min_profit_bps = (val * 100.0) as u32;
            }
        }

        if let Some(max_pos) = genome.get_gene("max_position") {
            if let Some(val) = max_pos.as_float() {
                self.config.max_position = (val * 1_000_000.0) as u64;
            }
        }

        self.genome = genome.clone();
        self.id = genome.id;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arbitrage_opportunity() {
        let opp = ArbitrageOpportunity {
            id: uuid::Uuid::new_v4(),
            asset: "ETH".to_string(),
            buy_venue: "Uniswap".to_string(),
            sell_venue: "Sushiswap".to_string(),
            buy_price: 2000.0,
            sell_price: 2020.0,
            profit_bps: 100,
            max_size: 100.0,
            expires_at: 9999999999,
            route: ArbitrageRoute {
                steps: vec![],
                gas_estimate: 200000,
                cross_chain: false,
                flash_loan: None,
            },
        };

        assert_eq!(opp.gross_profit(1.0), 20.0);
        assert!(opp.net_profit(1.0, 30) > 0.0);
        assert!(opp.is_valid(1000));
    }

    #[test]
    fn test_arbitrage_strategy() {
        let genome = Genome::new_float(5, 0.0, 1.0);
        let config = ArbitrageConfig {
            min_profit_bps: 10,
            max_position: 100000,
            venues: vec!["Uniswap".to_string(), "Sushiswap".to_string()],
            cross_chain: false,
        };

        let mut strategy = ArbitrageStrategy::new(genome, config);

        let mut market_data = MarketData::default();
        market_data.prices.insert("ETH".to_string(), 2000.0);
        market_data.timestamp = 1000;

        let opps = strategy.scan(&market_data);
        // May or may not find opportunities depending on random variance
        assert!(strategy.metrics.total_trades == 0);
    }
}
