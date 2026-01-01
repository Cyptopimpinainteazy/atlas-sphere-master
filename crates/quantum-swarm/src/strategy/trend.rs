//! Trend following strategy
//!
//! Implements technical analysis based trend following:
//! - Moving average crossovers
//! - Momentum indicators
//! - Breakout detection

use super::{MarketData, Signal, Strategy, StrategyMetrics, StrategyType, TrendConfig};
use crate::error::SwarmResult;
use crate::evolution::Genome;
use crate::types::{RiskProfile, StrategyId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Trend direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrendDirection {
    Up,
    Down,
    Sideways,
}

/// Trend signal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendSignal {
    /// Asset
    pub asset: String,
    /// Trend direction
    pub direction: TrendDirection,
    /// Signal strength (0-1)
    pub strength: f64,
    /// Entry price recommendation
    pub entry_price: Option<f64>,
    /// Indicators that triggered
    pub triggered_by: Vec<String>,
}

/// Trend indicator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendIndicator {
    /// Indicator name
    pub name: String,
    /// Parameters
    pub params: HashMap<String, f64>,
    /// Weight in combined signal
    pub weight: f64,
}

impl TrendIndicator {
    /// Simple Moving Average
    pub fn sma(period: usize) -> Self {
        Self {
            name: "SMA".to_string(),
            params: vec![("period".to_string(), period as f64)]
                .into_iter()
                .collect(),
            weight: 1.0,
        }
    }

    /// Exponential Moving Average
    pub fn ema(period: usize) -> Self {
        Self {
            name: "EMA".to_string(),
            params: vec![("period".to_string(), period as f64)]
                .into_iter()
                .collect(),
            weight: 1.0,
        }
    }

    /// RSI
    pub fn rsi(period: usize) -> Self {
        Self {
            name: "RSI".to_string(),
            params: vec![("period".to_string(), period as f64)]
                .into_iter()
                .collect(),
            weight: 1.0,
        }
    }

    /// MACD
    pub fn macd(fast: usize, slow: usize, signal: usize) -> Self {
        Self {
            name: "MACD".to_string(),
            params: vec![
                ("fast".to_string(), fast as f64),
                ("slow".to_string(), slow as f64),
                ("signal".to_string(), signal as f64),
            ]
            .into_iter()
            .collect(),
            weight: 1.0,
        }
    }
}

/// Price history for technical analysis
#[derive(Debug, Clone, Default)]
pub struct PriceHistory {
    /// Closing prices
    pub closes: Vec<f64>,
    /// Highs
    pub highs: Vec<f64>,
    /// Lows
    pub lows: Vec<f64>,
    /// Volumes
    pub volumes: Vec<f64>,
    /// Timestamps
    pub timestamps: Vec<u64>,
}

impl PriceHistory {
    /// Add a candle
    pub fn add(&mut self, close: f64, high: f64, low: f64, volume: f64, timestamp: u64) {
        self.closes.push(close);
        self.highs.push(high);
        self.lows.push(low);
        self.volumes.push(volume);
        self.timestamps.push(timestamp);

        // Keep last 500 candles
        if self.closes.len() > 500 {
            self.closes.remove(0);
            self.highs.remove(0);
            self.lows.remove(0);
            self.volumes.remove(0);
            self.timestamps.remove(0);
        }
    }

    /// Calculate SMA
    pub fn sma(&self, period: usize) -> Option<f64> {
        if self.closes.len() < period {
            return None;
        }
        let sum: f64 = self.closes.iter().rev().take(period).sum();
        Some(sum / period as f64)
    }

    /// Calculate EMA
    pub fn ema(&self, period: usize) -> Option<f64> {
        if self.closes.len() < period {
            return None;
        }
        let multiplier = 2.0 / (period as f64 + 1.0);
        let mut ema = self.sma(period)?;

        for price in self.closes.iter().rev().take(period) {
            ema = (price - ema) * multiplier + ema;
        }
        Some(ema)
    }

    /// Calculate RSI
    pub fn rsi(&self, period: usize) -> Option<f64> {
        if self.closes.len() < period + 1 {
            return None;
        }

        let mut gains = 0.0;
        let mut losses = 0.0;

        for i in (self.closes.len() - period)..self.closes.len() {
            let change = self.closes[i] - self.closes[i - 1];
            if change > 0.0 {
                gains += change;
            } else {
                losses -= change;
            }
        }

        let avg_gain = gains / period as f64;
        let avg_loss = losses / period as f64;

        if avg_loss == 0.0 {
            return Some(100.0);
        }

        let rs = avg_gain / avg_loss;
        Some(100.0 - (100.0 / (1.0 + rs)))
    }

    /// Calculate MACD
    pub fn macd(&self, fast: usize, slow: usize, signal: usize) -> Option<(f64, f64, f64)> {
        let fast_ema = self.ema(fast)?;
        let slow_ema = self.ema(slow)?;
        let macd_line = fast_ema - slow_ema;

        // Simplified signal line
        let signal_line = macd_line * 0.9; // Would need EMA of MACD
        let histogram = macd_line - signal_line;

        Some((macd_line, signal_line, histogram))
    }

    /// Last close price
    pub fn last_close(&self) -> Option<f64> {
        self.closes.last().copied()
    }
}

/// Trend following strategy
pub struct TrendStrategy {
    /// Strategy ID
    id: StrategyId,
    /// Genome parameters
    genome: Genome,
    /// Configuration
    config: TrendConfig,
    /// Strategy type
    strategy_type: StrategyType,
    /// Performance metrics
    metrics: StrategyMetrics,
    /// Indicators
    indicators: Vec<TrendIndicator>,
    /// Price history per asset
    history: HashMap<String, PriceHistory>,
    /// Current positions
    positions: HashMap<String, f64>,
    /// Risk profile
    risk: RiskProfile,
}

impl TrendStrategy {
    /// Create new trend strategy
    pub fn new(genome: Genome, config: TrendConfig) -> Self {
        // Build indicators from config
        let indicators = config
            .indicators
            .iter()
            .map(|name| match name.as_str() {
                "SMA" => TrendIndicator::sma(config.lookback),
                "EMA" => TrendIndicator::ema(config.lookback),
                "RSI" => TrendIndicator::rsi(14),
                "MACD" => TrendIndicator::macd(12, 26, 9),
                _ => TrendIndicator::sma(config.lookback),
            })
            .collect();

        Self {
            id: genome.id,
            strategy_type: StrategyType::TrendFollowing(config.clone()),
            genome,
            config,
            metrics: StrategyMetrics::default(),
            indicators,
            history: HashMap::new(),
            positions: HashMap::new(),
            risk: RiskProfile::default(),
        }
    }

    /// Update price history
    pub fn update_price(
        &mut self,
        asset: &str,
        close: f64,
        high: f64,
        low: f64,
        volume: f64,
        timestamp: u64,
    ) {
        self.history
            .entry(asset.to_string())
            .or_insert_with(PriceHistory::default)
            .add(close, high, low, volume, timestamp);
    }

    /// Analyze trend for asset
    pub fn analyze(&self, asset: &str) -> Option<TrendSignal> {
        let history = self.history.get(asset)?;

        if history.closes.len() < self.config.lookback {
            return None;
        }

        let mut bullish_signals = 0;
        let mut bearish_signals = 0;
        let mut triggered = Vec::new();

        // Check each indicator
        for indicator in &self.indicators {
            match indicator.name.as_str() {
                "SMA" | "EMA" => {
                    let ma = if indicator.name == "SMA" {
                        history.sma(self.config.lookback)?
                    } else {
                        history.ema(self.config.lookback)?
                    };

                    let current = history.last_close()?;
                    if current > ma * (1.0 + self.config.threshold / 100.0) {
                        bullish_signals += 1;
                        triggered.push(format!("{} crossover up", indicator.name));
                    } else if current < ma * (1.0 - self.config.threshold / 100.0) {
                        bearish_signals += 1;
                        triggered.push(format!("{} crossover down", indicator.name));
                    }
                }
                "RSI" => {
                    let rsi = history.rsi(14)?;
                    if rsi > 70.0 {
                        bearish_signals += 1;
                        triggered.push("RSI overbought".to_string());
                    } else if rsi < 30.0 {
                        bullish_signals += 1;
                        triggered.push("RSI oversold".to_string());
                    }
                }
                "MACD" => {
                    let (macd, signal, histogram) = history.macd(12, 26, 9)?;
                    if histogram > 0.0 && macd > signal {
                        bullish_signals += 1;
                        triggered.push("MACD bullish".to_string());
                    } else if histogram < 0.0 && macd < signal {
                        bearish_signals += 1;
                        triggered.push("MACD bearish".to_string());
                    }
                }
                _ => {}
            }
        }

        let total_signals = bullish_signals + bearish_signals;
        if total_signals == 0 {
            return None;
        }

        let (direction, strength) = if bullish_signals > bearish_signals {
            (
                TrendDirection::Up,
                bullish_signals as f64 / self.indicators.len() as f64,
            )
        } else if bearish_signals > bullish_signals {
            (
                TrendDirection::Down,
                bearish_signals as f64 / self.indicators.len() as f64,
            )
        } else {
            (TrendDirection::Sideways, 0.5)
        };

        Some(TrendSignal {
            asset: asset.to_string(),
            direction,
            strength,
            entry_price: history.last_close(),
            triggered_by: triggered,
        })
    }

    /// Get all trend signals
    pub fn all_signals(&self) -> Vec<TrendSignal> {
        self.history
            .keys()
            .filter_map(|asset| self.analyze(asset))
            .collect()
    }

    /// Update position
    pub fn update_position(&mut self, asset: &str, size: f64) {
        if size == 0.0 {
            self.positions.remove(asset);
        } else {
            self.positions.insert(asset.to_string(), size);
        }
    }

    /// Get current position
    pub fn position(&self, asset: &str) -> f64 {
        self.positions.get(asset).copied().unwrap_or(0.0)
    }
}

impl Strategy for TrendStrategy {
    fn id(&self) -> StrategyId {
        self.id
    }

    fn strategy_type(&self) -> &StrategyType {
        &self.strategy_type
    }

    fn genome(&self) -> &Genome {
        &self.genome
    }

    fn evaluate(&self, _market_data: &MarketData) -> SwarmResult<Vec<Signal>> {
        let mut signals = Vec::new();

        for trend_signal in self.all_signals() {
            if trend_signal.strength < self.config.threshold / 100.0 {
                continue;
            }

            match trend_signal.direction {
                TrendDirection::Up => {
                    signals.push(
                        Signal::buy(&trend_signal.asset, trend_signal.strength)
                            .with_urgency(trend_signal.strength),
                    );
                }
                TrendDirection::Down => {
                    signals.push(
                        Signal::sell(&trend_signal.asset, trend_signal.strength)
                            .with_urgency(trend_signal.strength),
                    );
                }
                TrendDirection::Sideways => {}
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
        // Update lookback from genome
        if let Some(lookback_gene) = genome.get_gene("lookback") {
            if let Some(val) = lookback_gene.as_float() {
                self.config.lookback = (val * 100.0) as usize;
            }
        }

        // Update threshold
        if let Some(threshold_gene) = genome.get_gene("threshold") {
            if let Some(val) = threshold_gene.as_float() {
                self.config.threshold = val * 10.0;
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
    fn test_price_history() {
        let mut history = PriceHistory::default();

        for i in 0..50 {
            history.add(
                100.0 + i as f64,
                101.0 + i as f64,
                99.0 + i as f64,
                1000.0,
                i as u64,
            );
        }

        let sma = history.sma(20).unwrap();
        assert!(sma > 100.0);

        let rsi = history.rsi(14).unwrap();
        assert!(rsi > 50.0); // Uptrend should have RSI > 50
    }

    #[test]
    fn test_trend_strategy() {
        let genome = Genome::new_float(3, 0.0, 1.0);
        let config = TrendConfig {
            lookback: 20,
            threshold: 1.0,
            indicators: vec!["SMA".to_string(), "RSI".to_string()],
        };

        let mut strategy = TrendStrategy::new(genome, config);

        // Add uptrending prices
        for i in 0..50 {
            strategy.update_price(
                "ETH",
                100.0 + i as f64,
                101.0 + i as f64,
                99.0 + i as f64,
                1000.0,
                i as u64,
            );
        }

        let signal = strategy.analyze("ETH");
        assert!(signal.is_some());

        let signal = signal.unwrap();
        assert_eq!(signal.direction, TrendDirection::Up);
    }

    #[test]
    fn test_indicators() {
        let sma = TrendIndicator::sma(20);
        assert_eq!(sma.name, "SMA");
        assert_eq!(sma.params.get("period"), Some(&20.0));

        let macd = TrendIndicator::macd(12, 26, 9);
        assert_eq!(macd.name, "MACD");
    }
}
