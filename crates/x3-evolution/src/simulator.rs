//! Strategy simulation using X3 VM

use crate::chromosome::Chromosome;
use crate::error::{EvolutionError, Result};
use crate::fitness::{FitnessEvaluator, FitnessScore};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Market data point for simulation
#[derive(Debug, Clone)]
pub struct MarketTick {
    /// Timestamp (unix ms)
    pub timestamp: u64,
    /// Asset symbol
    pub symbol: String,
    /// Price
    pub price: f64,
    /// Volume
    pub volume: f64,
    /// Bid price
    pub bid: f64,
    /// Ask price
    pub ask: f64,
    /// Additional data
    pub data: HashMap<String, f64>,
}

impl MarketTick {
    pub fn new(timestamp: u64, symbol: &str, price: f64) -> Self {
        Self {
            timestamp,
            symbol: symbol.to_string(),
            price,
            volume: 0.0,
            bid: price,
            ask: price,
            data: HashMap::new(),
        }
    }

    /// Spread as percentage
    pub fn spread_pct(&self) -> f64 {
        if self.bid > 0.0 {
            (self.ask - self.bid) / self.bid * 100.0
        } else {
            0.0
        }
    }
}

/// Trading action from strategy
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TradeAction {
    Buy,
    Sell,
    Hold,
}

/// Trade execution result
#[derive(Debug, Clone)]
pub struct Trade {
    pub timestamp: u64,
    pub action: TradeAction,
    pub price: f64,
    pub quantity: f64,
    pub fees: f64,
    pub slippage: f64,
}

impl Trade {
    pub fn total_cost(&self) -> f64 {
        match self.action {
            TradeAction::Buy => self.price * self.quantity + self.fees + self.slippage,
            TradeAction::Sell => -(self.price * self.quantity - self.fees - self.slippage),
            TradeAction::Hold => 0.0,
        }
    }
}

/// Portfolio state during simulation
#[derive(Debug, Clone)]
pub struct PortfolioState {
    /// Cash balance
    pub cash: f64,
    /// Asset positions (symbol -> quantity)
    pub positions: HashMap<String, f64>,
    /// Total value (cash + positions at current prices)
    pub total_value: f64,
    /// Peak value (for drawdown calculation)
    pub peak_value: f64,
    /// Trade history
    pub trades: Vec<Trade>,
    /// Daily returns
    pub daily_returns: Vec<f64>,
}

impl PortfolioState {
    pub fn new(initial_capital: f64) -> Self {
        Self {
            cash: initial_capital,
            positions: HashMap::new(),
            total_value: initial_capital,
            peak_value: initial_capital,
            trades: Vec::new(),
            daily_returns: Vec::new(),
        }
    }

    /// Calculate current drawdown percentage
    pub fn current_drawdown(&self) -> f64 {
        if self.peak_value > 0.0 {
            (self.peak_value - self.total_value) / self.peak_value * 100.0
        } else {
            0.0
        }
    }

    /// Update peak value
    pub fn update_peak(&mut self) {
        if self.total_value > self.peak_value {
            self.peak_value = self.total_value;
        }
    }

    /// Get position value
    pub fn position_value(&self, symbol: &str, price: f64) -> f64 {
        self.positions.get(symbol).copied().unwrap_or(0.0) * price
    }

    /// Update total value with current prices
    pub fn update_value(&mut self, prices: &HashMap<String, f64>) {
        let position_value: f64 = self
            .positions
            .iter()
            .map(|(sym, qty)| qty * prices.get(sym).copied().unwrap_or(0.0))
            .sum();

        self.total_value = self.cash + position_value;
        self.update_peak();
    }

    /// Record daily return
    pub fn record_daily_return(&mut self, previous_value: f64) {
        if previous_value > 0.0 {
            let ret = (self.total_value - previous_value) / previous_value;
            self.daily_returns.push(ret);
        }
    }

    /// Get PnL
    pub fn pnl(&self, initial_capital: f64) -> f64 {
        self.total_value - initial_capital
    }

    /// Get return percentage
    pub fn return_pct(&self, initial_capital: f64) -> f64 {
        if initial_capital > 0.0 {
            (self.total_value - initial_capital) / initial_capital * 100.0
        } else {
            0.0
        }
    }
}

/// Simulation configuration
#[derive(Debug, Clone)]
pub struct SimulationConfig {
    /// Initial capital
    pub initial_capital: f64,
    /// Trading fee rate (e.g., 0.001 for 0.1%)
    pub fee_rate: f64,
    /// Slippage rate (e.g., 0.0005 for 0.05%)
    pub slippage_rate: f64,
    /// Maximum position size as fraction of portfolio
    pub max_position_size: f64,
    /// Risk-free rate for Sharpe calculation (annualized)
    pub risk_free_rate: f64,
    /// Maximum simulation time
    pub timeout: Duration,
    /// Use realistic execution (delays, partial fills)
    pub realistic_execution: bool,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            initial_capital: 100_000.0,
            fee_rate: 0.001,
            slippage_rate: 0.0005,
            max_position_size: 0.25,
            risk_free_rate: 0.02,
            timeout: Duration::from_secs(30),
            realistic_execution: false,
        }
    }
}

/// Simulation result
#[derive(Debug, Clone)]
pub struct SimulationResult {
    /// Final portfolio state
    pub portfolio: PortfolioState,
    /// Fitness score
    pub fitness: FitnessScore,
    /// Total number of ticks processed
    pub ticks_processed: usize,
    /// Simulation duration
    pub duration: Duration,
    /// Errors encountered
    pub errors: Vec<String>,
}

/// Strategy simulator
pub struct Simulator {
    config: SimulationConfig,
}

impl Simulator {
    pub fn new(config: SimulationConfig) -> Self {
        Self { config }
    }

    /// Run simulation with chromosome against market data
    pub fn simulate(
        &self,
        chromosome: &Chromosome,
        market_data: &[MarketTick],
    ) -> Result<SimulationResult> {
        let start = Instant::now();
        let mut portfolio = PortfolioState::new(self.config.initial_capital);
        let mut errors = Vec::new();
        let mut ticks_processed = 0;
        let mut previous_value = self.config.initial_capital;
        let mut last_day: Option<u64> = None;

        // Execute strategy for each tick
        for tick in market_data {
            // Check timeout
            if start.elapsed() > self.config.timeout {
                errors.push(format!(
                    "Simulation timeout after {} ticks",
                    ticks_processed
                ));
                break;
            }

            // Record daily returns (simplified: every 1000 ticks = 1 day)
            let current_day = tick.timestamp / (24 * 60 * 60 * 1000);
            if last_day.is_some() && last_day != Some(current_day) {
                portfolio.record_daily_return(previous_value);
                previous_value = portfolio.total_value;
            }
            last_day = Some(current_day);

            // Execute strategy bytecode to get action
            let action = self.execute_strategy(chromosome, tick, &portfolio)?;

            // Apply action
            if let Some(trade) = self.execute_trade(action, tick, &mut portfolio)? {
                portfolio.trades.push(trade);
            }

            // Update portfolio value
            let mut prices = HashMap::new();
            prices.insert(tick.symbol.clone(), tick.price);
            portfolio.update_value(&prices);

            ticks_processed += 1;
        }

        // Calculate final fitness
        let fitness = self.calculate_fitness(&portfolio);

        Ok(SimulationResult {
            portfolio,
            fitness,
            ticks_processed,
            duration: start.elapsed(),
            errors,
        })
    }

    /// Execute strategy bytecode to determine action
    fn execute_strategy(
        &self,
        chromosome: &Chromosome,
        tick: &MarketTick,
        portfolio: &PortfolioState,
    ) -> Result<TradeAction> {
        // In a real implementation, this would:
        // 1. Initialize X3 VM with strategy bytecode
        // 2. Set up input memory with tick data
        // 3. Execute bytecode
        // 4. Read output (trading decision)

        // For now, use chromosome parameters to make decision
        let bytecode = chromosome.to_bytecode();

        if bytecode.is_empty() {
            return Ok(TradeAction::Hold);
        }

        // Simple decision based on bytecode parameters
        // This is a placeholder - real implementation uses X3 VM
        let decision_byte = bytecode.get(1).copied().unwrap_or(128);
        let threshold = bytecode.get(2).copied().unwrap_or(128);

        // Calculate a simple signal from price
        let price_signal = ((tick.price * 1000.0) as u64 % 256) as u8;

        // Position check
        let has_position = portfolio
            .positions
            .get(&tick.symbol)
            .map(|&q| q > 0.0)
            .unwrap_or(false);

        let action = if price_signal > threshold && !has_position {
            // Buy signal
            TradeAction::Buy
        } else if price_signal < decision_byte && has_position {
            // Sell signal
            TradeAction::Sell
        } else {
            TradeAction::Hold
        };

        Ok(action)
    }

    /// Execute a trade based on action
    fn execute_trade(
        &self,
        action: TradeAction,
        tick: &MarketTick,
        portfolio: &mut PortfolioState,
    ) -> Result<Option<Trade>> {
        match action {
            TradeAction::Hold => Ok(None),

            TradeAction::Buy => {
                // Calculate position size (max 25% of portfolio)
                let max_value = portfolio.cash * self.config.max_position_size;
                let price_with_slippage = tick.ask * (1.0 + self.config.slippage_rate);
                let quantity = max_value / price_with_slippage;

                if quantity <= 0.0 || max_value < 100.0 {
                    return Ok(None);
                }

                let fees = max_value * self.config.fee_rate;
                let slippage = max_value * self.config.slippage_rate;
                let total_cost = max_value + fees;

                if total_cost > portfolio.cash {
                    return Ok(None);
                }

                // Execute
                portfolio.cash -= total_cost;
                *portfolio
                    .positions
                    .entry(tick.symbol.clone())
                    .or_insert(0.0) += quantity;

                Ok(Some(Trade {
                    timestamp: tick.timestamp,
                    action: TradeAction::Buy,
                    price: price_with_slippage,
                    quantity,
                    fees,
                    slippage,
                }))
            }

            TradeAction::Sell => {
                let quantity = portfolio
                    .positions
                    .get(&tick.symbol)
                    .copied()
                    .unwrap_or(0.0);

                if quantity <= 0.0 {
                    return Ok(None);
                }

                let price_with_slippage = tick.bid * (1.0 - self.config.slippage_rate);
                let proceeds = quantity * price_with_slippage;
                let fees = proceeds * self.config.fee_rate;
                let slippage = quantity * tick.bid * self.config.slippage_rate;

                // Execute
                portfolio.cash += proceeds - fees;
                portfolio.positions.remove(&tick.symbol);

                Ok(Some(Trade {
                    timestamp: tick.timestamp,
                    action: TradeAction::Sell,
                    price: price_with_slippage,
                    quantity,
                    fees,
                    slippage,
                }))
            }
        }
    }

    /// Calculate fitness from simulation results
    fn calculate_fitness(&self, portfolio: &PortfolioState) -> FitnessScore {
        let pnl = portfolio.pnl(self.config.initial_capital);
        let return_pct = portfolio.return_pct(self.config.initial_capital);

        // Calculate Sharpe ratio
        let sharpe = self.calculate_sharpe(&portfolio.daily_returns);

        // Calculate Sortino ratio
        let sortino = self.calculate_sortino(&portfolio.daily_returns);

        // Calculate max drawdown
        let max_drawdown = self.calculate_max_drawdown(&portfolio.daily_returns);

        // Win rate
        let win_rate = self.calculate_win_rate(&portfolio.trades);

        // Average trade
        let avg_trade = if !portfolio.trades.is_empty() {
            pnl / portfolio.trades.len() as f64
        } else {
            0.0
        };

        FitnessScore {
            pnl,
            sharpe_ratio: sharpe,
            sortino_ratio: sortino,
            max_drawdown,
            win_rate,
            profit_factor: if pnl > 0.0 { 1.5 } else { 0.5 }, // Simplified
            avg_trade,
            avg_duration: 0.0,
            trade_count: portfolio.trades.len(),
            calmar_ratio: if max_drawdown > 0.01 {
                return_pct / max_drawdown
            } else {
                return_pct
            },
            volatility: self.calculate_volatility(&portfolio.daily_returns),
            custom: 0.0,
        }
    }

    fn calculate_sharpe(&self, daily_returns: &[f64]) -> f64 {
        if daily_returns.len() < 2 {
            return 0.0;
        }

        let mean: f64 = daily_returns.iter().sum::<f64>() / daily_returns.len() as f64;
        let annualized_mean = mean * 252.0;

        let variance: f64 = daily_returns
            .iter()
            .map(|r| (r - mean).powi(2))
            .sum::<f64>()
            / (daily_returns.len() - 1) as f64;
        let std_dev = variance.sqrt() * (252.0_f64).sqrt();

        if std_dev > 0.0 {
            (annualized_mean - self.config.risk_free_rate) / std_dev
        } else {
            0.0
        }
    }

    fn calculate_sortino(&self, daily_returns: &[f64]) -> f64 {
        if daily_returns.len() < 2 {
            return 0.0;
        }

        let mean: f64 = daily_returns.iter().sum::<f64>() / daily_returns.len() as f64;
        let annualized_mean = mean * 252.0;

        let downside_variance: f64 = daily_returns
            .iter()
            .filter(|&&r| r < 0.0)
            .map(|r| r.powi(2))
            .sum::<f64>()
            / daily_returns.len() as f64;
        let downside_dev = downside_variance.sqrt() * (252.0_f64).sqrt();

        if downside_dev > 0.0 {
            (annualized_mean - self.config.risk_free_rate) / downside_dev
        } else {
            annualized_mean
        }
    }

    fn calculate_max_drawdown(&self, daily_returns: &[f64]) -> f64 {
        if daily_returns.is_empty() {
            return 0.0;
        }

        let mut cumulative = 1.0;
        let mut peak = 1.0;
        let mut max_dd = 0.0;

        for &ret in daily_returns {
            cumulative *= 1.0 + ret;
            if cumulative > peak {
                peak = cumulative;
            }
            let dd = (peak - cumulative) / peak;
            if dd > max_dd {
                max_dd = dd;
            }
        }

        max_dd * 100.0 // As percentage
    }

    fn calculate_win_rate(&self, trades: &[Trade]) -> f64 {
        if trades.is_empty() {
            return 0.0;
        }

        let mut wins = 0;
        let mut prev_sell_price: Option<f64> = None;

        for trade in trades {
            match trade.action {
                TradeAction::Buy => {
                    prev_sell_price = None;
                }
                TradeAction::Sell => {
                    if let Some(buy_price) = prev_sell_price {
                        if trade.price > buy_price {
                            wins += 1;
                        }
                    }
                    prev_sell_price = Some(trade.price);
                }
                _ => {}
            }
        }

        // Simplified: assume 50% of closed trades are wins
        let closed_trades = trades
            .iter()
            .filter(|t| matches!(t.action, TradeAction::Sell))
            .count();

        if closed_trades > 0 {
            (closed_trades / 2) as f64 / closed_trades as f64
        } else {
            0.0
        }
    }

    fn calculate_volatility(&self, daily_returns: &[f64]) -> f64 {
        if daily_returns.len() < 2 {
            return 0.0;
        }

        let mean: f64 = daily_returns.iter().sum::<f64>() / daily_returns.len() as f64;
        let variance: f64 = daily_returns
            .iter()
            .map(|r| (r - mean).powi(2))
            .sum::<f64>()
            / (daily_returns.len() - 1) as f64;

        variance.sqrt() * (252.0_f64).sqrt() * 100.0 // Annualized percentage
    }
}

/// Fitness evaluator using simulation
pub struct SimulatorFitness {
    simulator: Simulator,
    market_data: Vec<MarketTick>,
}

impl SimulatorFitness {
    pub fn new(config: SimulationConfig, market_data: Vec<MarketTick>) -> Self {
        Self {
            simulator: Simulator::new(config),
            market_data,
        }
    }
}

impl FitnessEvaluator for SimulatorFitness {
    fn evaluate(&self, chromosome: &Chromosome) -> Result<FitnessScore> {
        let result = self.simulator.simulate(chromosome, &self.market_data)?;
        Ok(result.fitness)
    }

    fn name(&self) -> &'static str {
        "SimulatorFitness"
    }
}

/// Generate synthetic market data for testing
pub fn generate_synthetic_data(
    symbol: &str,
    num_ticks: usize,
    initial_price: f64,
    volatility: f64,
) -> Vec<MarketTick> {
    use rand::Rng;
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;

    let mut rng = ChaCha20Rng::from_entropy();
    let mut ticks = Vec::with_capacity(num_ticks);
    let mut price = initial_price;
    let mut timestamp = 1704067200000u64; // 2024-01-01 00:00:00 UTC

    for _ in 0..num_ticks {
        // Random walk
        let change = rng.gen::<f64>() * 2.0 - 1.0; // -1 to 1
        price *= 1.0 + change * volatility;
        price = price.max(0.01); // Floor

        let spread = price * 0.001; // 0.1% spread

        let mut tick = MarketTick {
            timestamp,
            symbol: symbol.to_string(),
            price,
            volume: rng.gen::<f64>() * 1000.0,
            bid: price - spread / 2.0,
            ask: price + spread / 2.0,
            data: HashMap::new(),
        };

        tick.data
            .insert("rsi".to_string(), rng.gen::<f64>() * 100.0);
        tick.data
            .insert("macd".to_string(), rng.gen::<f64>() * 2.0 - 1.0);

        ticks.push(tick);
        timestamp += 60_000; // 1 minute
    }

    ticks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_portfolio_state() {
        let mut portfolio = PortfolioState::new(100_000.0);
        assert_eq!(portfolio.cash, 100_000.0);
        assert_eq!(portfolio.total_value, 100_000.0);

        portfolio.cash -= 10_000.0;
        portfolio.positions.insert("BTC".to_string(), 1.0);

        let mut prices = HashMap::new();
        prices.insert("BTC".to_string(), 50_000.0);
        portfolio.update_value(&prices);

        assert_eq!(portfolio.total_value, 140_000.0);
    }

    #[test]
    fn test_synthetic_data_generation() {
        let data = generate_synthetic_data("BTC", 1000, 50000.0, 0.01);
        assert_eq!(data.len(), 1000);

        for tick in &data {
            assert!(tick.price > 0.0);
            assert!(tick.bid < tick.ask);
        }
    }

    #[test]
    fn test_simulation() {
        let config = SimulationConfig::default();
        let simulator = Simulator::new(config);

        let bytecode = vec![0x20, 128, 128, 0x00];
        let chromosome = Chromosome::from_bytecode(bytecode).unwrap();

        let data = generate_synthetic_data("ETH", 100, 2000.0, 0.02);

        let result = simulator.simulate(&chromosome, &data).unwrap();
        assert!(result.ticks_processed > 0);
    }

    #[test]
    fn test_trade_execution() {
        let config = SimulationConfig::default();
        let simulator = Simulator::new(config);

        let mut portfolio = PortfolioState::new(100_000.0);
        let tick = MarketTick::new(0, "ETH", 2000.0);

        // Execute buy
        let trade = simulator
            .execute_trade(TradeAction::Buy, &tick, &mut portfolio)
            .unwrap()
            .unwrap();

        assert_eq!(trade.action, TradeAction::Buy);
        assert!(portfolio.cash < 100_000.0);
        assert!(portfolio.positions.contains_key("ETH"));
    }

    #[test]
    fn test_fitness_calculation() {
        let daily_returns = vec![0.01, -0.005, 0.02, 0.015, -0.01, 0.005];

        let config = SimulationConfig::default();
        let simulator = Simulator::new(config);

        let sharpe = simulator.calculate_sharpe(&daily_returns);
        let sortino = simulator.calculate_sortino(&daily_returns);
        let max_dd = simulator.calculate_max_drawdown(&daily_returns);
        let volatility = simulator.calculate_volatility(&daily_returns);

        // Just verify they compute without panicking
        assert!(sharpe.is_finite());
        assert!(sortino.is_finite());
        assert!(max_dd >= 0.0);
        assert!(volatility >= 0.0);
    }
}
