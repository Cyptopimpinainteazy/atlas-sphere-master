//! Strategy module for financial trading strategies
//!
//! Implements various strategy types:
//! - Arbitrage strategies (cross-VM, cross-chain)
//! - Portfolio optimization strategies
//! - Market making strategies
//! - Trend following strategies

mod arbitrage;
mod market_making;
mod portfolio;
mod trend;

pub use arbitrage::{ArbitrageOpportunity, ArbitrageRoute, ArbitrageStrategy};
pub use market_making::{InventoryTarget, MarketMakingStrategy, SpreadConfig};
pub use portfolio::{PortfolioAllocation, PortfolioStrategy, RebalanceSignal};
pub use trend::{TrendIndicator, TrendSignal, TrendStrategy};

use crate::error::{SwarmError, SwarmResult};
use crate::evolution::Genome;
use crate::types::{RiskProfile, StrategyId, TradeRoute};
use serde::{Deserialize, Serialize};

/// Strategy type enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StrategyType {
    /// Cross-market arbitrage
    Arbitrage(ArbitrageConfig),
    /// Portfolio optimization
    Portfolio(PortfolioConfig),
    /// Market making
    MarketMaking(MarketMakingConfig),
    /// Trend following
    TrendFollowing(TrendConfig),
    /// Custom/hybrid
    Custom { name: String },
}

/// Arbitrage strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageConfig {
    /// Minimum profit threshold (basis points)
    pub min_profit_bps: u32,
    /// Maximum position size
    pub max_position: u64,
    /// Allowed venues
    pub venues: Vec<String>,
    /// Enable cross-chain
    pub cross_chain: bool,
}

/// Portfolio strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioConfig {
    /// Target assets
    pub assets: Vec<String>,
    /// Rebalance threshold (%)
    pub rebalance_threshold: f64,
    /// Risk tolerance (0-1)
    pub risk_tolerance: f64,
    /// Optimization method
    pub optimization: OptimizationMethod,
}

/// Portfolio optimization method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationMethod {
    /// Mean-variance (Markowitz)
    MeanVariance,
    /// Risk parity
    RiskParity,
    /// Maximum Sharpe ratio
    MaxSharpe,
    /// Minimum variance
    MinVariance,
    /// Black-Litterman
    BlackLitterman,
}

/// Market making strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketMakingConfig {
    /// Target spread (basis points)
    pub target_spread_bps: u32,
    /// Inventory limits
    pub inventory_limits: (i64, i64),
    /// Quote refresh rate (ms)
    pub refresh_rate_ms: u64,
    /// Enable dynamic pricing
    pub dynamic_pricing: bool,
}

/// Trend following configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendConfig {
    /// Lookback period
    pub lookback: usize,
    /// Signal threshold
    pub threshold: f64,
    /// Indicators to use
    pub indicators: Vec<String>,
}

/// Base strategy trait
pub trait Strategy: Send + Sync {
    /// Get strategy ID
    fn id(&self) -> StrategyId;

    /// Get strategy type
    fn strategy_type(&self) -> &StrategyType;

    /// Get underlying genome
    fn genome(&self) -> &Genome;

    /// Evaluate current market state and produce signals
    fn evaluate(&self, market_data: &MarketData) -> SwarmResult<Vec<Signal>>;

    /// Get risk profile
    fn risk_profile(&self) -> RiskProfile;

    /// Get current performance metrics
    fn metrics(&self) -> StrategyMetrics;

    /// Update from genome parameters
    fn update_from_genome(&mut self, genome: &Genome);
}

/// Market data snapshot
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MarketData {
    /// Asset prices
    pub prices: std::collections::HashMap<String, f64>,
    /// Order book depths
    pub depths: std::collections::HashMap<String, (f64, f64)>,
    /// 24h volumes
    pub volumes: std::collections::HashMap<String, f64>,
    /// Timestamp
    pub timestamp: u64,
}

/// Trading signal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    /// Asset
    pub asset: String,
    /// Direction (positive = buy, negative = sell)
    pub direction: f64,
    /// Confidence (0-1)
    pub confidence: f64,
    /// Target price
    pub target_price: Option<f64>,
    /// Stop loss
    pub stop_loss: Option<f64>,
    /// Take profit
    pub take_profit: Option<f64>,
    /// Urgency (0-1)
    pub urgency: f64,
}

impl Signal {
    /// Create buy signal
    pub fn buy(asset: &str, confidence: f64) -> Self {
        Self {
            asset: asset.to_string(),
            direction: 1.0,
            confidence,
            target_price: None,
            stop_loss: None,
            take_profit: None,
            urgency: 0.5,
        }
    }

    /// Create sell signal
    pub fn sell(asset: &str, confidence: f64) -> Self {
        Self {
            asset: asset.to_string(),
            direction: -1.0,
            confidence,
            target_price: None,
            stop_loss: None,
            take_profit: None,
            urgency: 0.5,
        }
    }

    /// Add stop loss
    pub fn with_stop_loss(mut self, price: f64) -> Self {
        self.stop_loss = Some(price);
        self
    }

    /// Add take profit
    pub fn with_take_profit(mut self, price: f64) -> Self {
        self.take_profit = Some(price);
        self
    }

    /// Set urgency
    pub fn with_urgency(mut self, urgency: f64) -> Self {
        self.urgency = urgency.clamp(0.0, 1.0);
        self
    }

    /// Is this a buy signal
    pub fn is_buy(&self) -> bool {
        self.direction > 0.0
    }

    /// Is this a sell signal
    pub fn is_sell(&self) -> bool {
        self.direction < 0.0
    }
}

/// Strategy performance metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StrategyMetrics {
    /// Total PnL
    pub total_pnl: f64,
    /// Total trades
    pub total_trades: usize,
    /// Win rate
    pub win_rate: f64,
    /// Sharpe ratio
    pub sharpe_ratio: f64,
    /// Max drawdown
    pub max_drawdown: f64,
    /// Average trade duration (seconds)
    pub avg_trade_duration: f64,
    /// Total volume traded
    pub total_volume: f64,
}

impl StrategyMetrics {
    /// Calculate profit factor
    pub fn profit_factor(&self) -> f64 {
        if self.total_pnl < 0.0 {
            0.0
        } else {
            1.0 + self.total_pnl / self.total_volume.max(1.0)
        }
    }

    /// Calculate risk-adjusted return
    pub fn risk_adjusted_return(&self) -> f64 {
        if self.max_drawdown.abs() < 0.001 {
            self.total_pnl
        } else {
            self.total_pnl / self.max_drawdown.abs()
        }
    }
}

/// Strategy factory for creating strategies from genomes
pub struct StrategyFactory;

impl StrategyFactory {
    /// Create arbitrage strategy from genome
    pub fn create_arbitrage(genome: Genome, config: ArbitrageConfig) -> ArbitrageStrategy {
        ArbitrageStrategy::new(genome, config)
    }

    /// Create portfolio strategy from genome
    pub fn create_portfolio(genome: Genome, config: PortfolioConfig) -> PortfolioStrategy {
        PortfolioStrategy::new(genome, config)
    }

    /// Create market making strategy from genome
    pub fn create_market_making(
        genome: Genome,
        config: MarketMakingConfig,
    ) -> MarketMakingStrategy {
        MarketMakingStrategy::new(genome, config)
    }

    /// Create trend following strategy from genome
    pub fn create_trend(genome: Genome, config: TrendConfig) -> TrendStrategy {
        TrendStrategy::new(genome, config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_creation() {
        let signal = Signal::buy("ETH", 0.8)
            .with_stop_loss(1800.0)
            .with_take_profit(2200.0)
            .with_urgency(0.9);

        assert!(signal.is_buy());
        assert!(!signal.is_sell());
        assert_eq!(signal.confidence, 0.8);
        assert_eq!(signal.stop_loss, Some(1800.0));
        assert_eq!(signal.take_profit, Some(2200.0));
        assert_eq!(signal.urgency, 0.9);
    }

    #[test]
    fn test_strategy_metrics() {
        let metrics = StrategyMetrics {
            total_pnl: 1000.0,
            total_trades: 50,
            win_rate: 0.6,
            sharpe_ratio: 1.5,
            max_drawdown: -200.0,
            avg_trade_duration: 3600.0,
            total_volume: 100000.0,
        };

        assert!(metrics.profit_factor() > 1.0);
        assert!(metrics.risk_adjusted_return() > 0.0);
    }
}
