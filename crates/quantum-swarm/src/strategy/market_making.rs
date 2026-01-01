//! Market making strategy
//!
//! Implements automated market making:
//! - Dynamic spread calculation
//! - Inventory management
//! - Quote optimization

use super::{MarketData, MarketMakingConfig, Signal, Strategy, StrategyMetrics, StrategyType};
use crate::error::SwarmResult;
use crate::evolution::Genome;
use crate::types::{RiskProfile, StrategyId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Spread configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpreadConfig {
    /// Minimum spread (basis points)
    pub min_spread_bps: u32,
    /// Maximum spread (basis points)
    pub max_spread_bps: u32,
    /// Volatility multiplier
    pub volatility_multiplier: f64,
    /// Inventory skew factor
    pub inventory_skew: f64,
}

impl Default for SpreadConfig {
    fn default() -> Self {
        Self {
            min_spread_bps: 5,
            max_spread_bps: 100,
            volatility_multiplier: 2.0,
            inventory_skew: 0.5,
        }
    }
}

/// Inventory management target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryTarget {
    /// Target inventory level
    pub target: f64,
    /// Maximum long position
    pub max_long: f64,
    /// Maximum short position
    pub max_short: f64,
    /// Current inventory
    pub current: f64,
}

impl InventoryTarget {
    /// Get inventory ratio (-1 to 1)
    pub fn ratio(&self) -> f64 {
        if self.max_long > 0.0 {
            (self.current - self.target) / self.max_long
        } else {
            0.0
        }
    }

    /// Is inventory within limits
    pub fn within_limits(&self) -> bool {
        self.current >= -self.max_short && self.current <= self.max_long
    }
}

/// A quote (bid or ask)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    /// Price
    pub price: f64,
    /// Size
    pub size: f64,
    /// Side (bid or ask)
    pub side: QuoteSide,
}

/// Quote side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuoteSide {
    Bid,
    Ask,
}

/// Market making strategy
pub struct MarketMakingStrategy {
    /// Strategy ID
    id: StrategyId,
    /// Genome parameters
    genome: Genome,
    /// Configuration
    config: MarketMakingConfig,
    /// Strategy type
    strategy_type: StrategyType,
    /// Performance metrics
    metrics: StrategyMetrics,
    /// Spread config
    spread: SpreadConfig,
    /// Inventory per asset
    inventory: HashMap<String, InventoryTarget>,
    /// Active quotes
    quotes: Vec<Quote>,
    /// Risk profile
    risk: RiskProfile,
    /// Volatility estimates
    volatility: HashMap<String, f64>,
}

impl MarketMakingStrategy {
    /// Create new market making strategy
    pub fn new(genome: Genome, config: MarketMakingConfig) -> Self {
        Self {
            id: genome.id,
            strategy_type: StrategyType::MarketMaking(config.clone()),
            genome,
            config,
            metrics: StrategyMetrics::default(),
            spread: SpreadConfig::default(),
            inventory: HashMap::new(),
            quotes: Vec::new(),
            risk: RiskProfile::default(),
            volatility: HashMap::new(),
        }
    }

    /// Update volatility estimate
    pub fn update_volatility(&mut self, asset: &str, vol: f64) {
        self.volatility.insert(asset.to_string(), vol);
    }

    /// Calculate optimal spread
    pub fn calculate_spread(&self, asset: &str, mid_price: f64) -> (f64, f64) {
        let base_spread_bps = self.config.target_spread_bps as f64;
        let vol = self.volatility.get(asset).copied().unwrap_or(0.01);

        // Adjust for volatility
        let vol_adjusted = base_spread_bps * (1.0 + vol * self.spread.volatility_multiplier);

        // Adjust for inventory
        let inventory_ratio = self.inventory.get(asset).map(|i| i.ratio()).unwrap_or(0.0);

        // Skew spreads based on inventory
        // If long, widen ask and tighten bid to encourage selling
        let bid_adjustment = 1.0 - inventory_ratio * self.spread.inventory_skew;
        let ask_adjustment = 1.0 + inventory_ratio * self.spread.inventory_skew;

        let half_spread = (vol_adjusted / 2.0) / 10000.0 * mid_price;

        let bid_spread = (half_spread * bid_adjustment)
            .max((self.spread.min_spread_bps as f64 / 2.0) / 10000.0 * mid_price)
            .min((self.spread.max_spread_bps as f64 / 2.0) / 10000.0 * mid_price);

        let ask_spread = (half_spread * ask_adjustment)
            .max((self.spread.min_spread_bps as f64 / 2.0) / 10000.0 * mid_price)
            .min((self.spread.max_spread_bps as f64 / 2.0) / 10000.0 * mid_price);

        (bid_spread, ask_spread)
    }

    /// Generate quotes
    pub fn generate_quotes(
        &mut self,
        asset: &str,
        mid_price: f64,
        base_size: f64,
    ) -> (Quote, Quote) {
        let (bid_spread, ask_spread) = self.calculate_spread(asset, mid_price);

        // Adjust size based on inventory
        let inventory_ratio = self.inventory.get(asset).map(|i| i.ratio()).unwrap_or(0.0);

        // If long, offer more on ask side
        let bid_size = base_size * (1.0 - inventory_ratio * 0.5).max(0.1);
        let ask_size = base_size * (1.0 + inventory_ratio * 0.5).max(0.1);

        let bid = Quote {
            price: mid_price - bid_spread,
            size: bid_size,
            side: QuoteSide::Bid,
        };

        let ask = Quote {
            price: mid_price + ask_spread,
            size: ask_size,
            side: QuoteSide::Ask,
        };

        self.quotes = vec![bid.clone(), ask.clone()];

        (bid, ask)
    }

    /// Update inventory
    pub fn update_inventory(&mut self, asset: &str, delta: f64) {
        let entry = self
            .inventory
            .entry(asset.to_string())
            .or_insert(InventoryTarget {
                target: 0.0,
                max_long: self.config.inventory_limits.1 as f64,
                max_short: self.config.inventory_limits.0.abs() as f64,
                current: 0.0,
            });
        entry.current += delta;
    }

    /// Check if should quote
    pub fn should_quote(&self, asset: &str) -> bool {
        self.inventory
            .get(asset)
            .map(|i| i.within_limits())
            .unwrap_or(true)
    }

    /// Get current quotes
    pub fn current_quotes(&self) -> &[Quote] {
        &self.quotes
    }

    /// Calculate PnL from fill
    pub fn on_fill(&mut self, price: f64, size: f64, side: QuoteSide) {
        let pnl = match side {
            QuoteSide::Bid => {
                // Bought - negative cash, positive inventory
                -price * size
            }
            QuoteSide::Ask => {
                // Sold - positive cash, negative inventory
                price * size
            }
        };

        self.metrics.total_pnl += pnl;
        self.metrics.total_trades += 1;
        self.metrics.total_volume += price * size;
    }
}

impl Strategy for MarketMakingStrategy {
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

        // Generate signals based on inventory imbalance
        for (asset, inv) in &self.inventory {
            let ratio = inv.ratio();

            if ratio > 0.5 {
                // Too long, sell
                signals.push(Signal::sell(asset, ratio));
            } else if ratio < -0.5 {
                // Too short, buy
                signals.push(Signal::buy(asset, -ratio));
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
        // Update spread parameters from genome
        if let Some(spread_gene) = genome.get_gene("spread") {
            if let Some(val) = spread_gene.as_float() {
                self.config.target_spread_bps = (val * 100.0) as u32;
            }
        }

        if let Some(skew_gene) = genome.get_gene("inventory_skew") {
            if let Some(val) = skew_gene.as_float() {
                self.spread.inventory_skew = val;
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
    fn test_spread_calculation() {
        let genome = Genome::new_float(5, 0.0, 1.0);
        let config = MarketMakingConfig {
            target_spread_bps: 20,
            inventory_limits: (-100, 100),
            refresh_rate_ms: 100,
            dynamic_pricing: true,
        };

        let strategy = MarketMakingStrategy::new(genome, config);

        let (bid_spread, ask_spread) = strategy.calculate_spread("ETH", 2000.0);

        assert!(bid_spread > 0.0);
        assert!(ask_spread > 0.0);
    }

    #[test]
    fn test_quote_generation() {
        let genome = Genome::new_float(5, 0.0, 1.0);
        let config = MarketMakingConfig {
            target_spread_bps: 20,
            inventory_limits: (-100, 100),
            refresh_rate_ms: 100,
            dynamic_pricing: true,
        };

        let mut strategy = MarketMakingStrategy::new(genome, config);

        let (bid, ask) = strategy.generate_quotes("ETH", 2000.0, 1.0);

        assert!(bid.price < 2000.0);
        assert!(ask.price > 2000.0);
        assert_eq!(bid.side, QuoteSide::Bid);
        assert_eq!(ask.side, QuoteSide::Ask);
    }

    #[test]
    fn test_inventory_management() {
        let genome = Genome::new_float(5, 0.0, 1.0);
        let config = MarketMakingConfig {
            target_spread_bps: 20,
            inventory_limits: (-100, 100),
            refresh_rate_ms: 100,
            dynamic_pricing: true,
        };

        let mut strategy = MarketMakingStrategy::new(genome, config);

        strategy.update_inventory("ETH", 50.0);

        let inv = strategy.inventory.get("ETH").unwrap();
        assert_eq!(inv.current, 50.0);
        assert!(inv.within_limits());
    }
}
