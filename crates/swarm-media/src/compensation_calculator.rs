//! Compensation Calculator Module
//!
//! Tier-based compensation calculation with bonuses for GPU contributors
//! and marketing agents in the swarm network.

use serde::{Deserialize, Serialize};

/// Compensation tier levels for contributors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompensationTier {
    /// Entry level contributor
    Bronze,
    /// Established contributor
    Silver,
    /// High-performance contributor
    Gold,
    /// Elite contributor
    Platinum,
}

impl Default for CompensationTier {
    fn default() -> Self {
        Self::Bronze
    }
}

impl CompensationTier {
    /// Get the multiplier for this tier (as percentage, e.g., 150 = 1.5x)
    pub fn multiplier(&self) -> u32 {
        match self {
            CompensationTier::Bronze => 100,
            CompensationTier::Silver => 125,
            CompensationTier::Gold => 150,
            CompensationTier::Platinum => 200,
        }
    }
}

/// Compensation calculation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompensationParams {
    /// Base rate per compute unit (in smallest denomination)
    pub base_rate: u128,
    /// Performance bonus percentage (0-100)
    pub performance_bonus_pct: u8,
    /// Loyalty bonus for sustained contributions
    pub loyalty_bonus: u128,
    /// Minimum payout threshold
    pub min_payout: u128,
}

impl Default for CompensationParams {
    fn default() -> Self {
        Self {
            base_rate: 1_000_000_000_000, // 1 unit base (12 decimals)
            performance_bonus_pct: 0,
            loyalty_bonus: 0,
            min_payout: 100_000_000_000, // 0.1 units minimum
        }
    }
}

/// Result of a compensation calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompensationResult {
    /// Base amount before bonuses
    pub base_amount: u128,
    /// Tier bonus amount
    pub tier_bonus: u128,
    /// Performance bonus amount
    pub performance_bonus: u128,
    /// Loyalty bonus amount
    pub loyalty_bonus: u128,
    /// Total compensation
    pub total: u128,
    /// Whether payout meets minimum threshold
    pub meets_minimum: bool,
}

/// Calculate compensation for a contributor
pub fn calculate_compensation(
    compute_units: u128,
    tier: CompensationTier,
    params: &CompensationParams,
) -> CompensationResult {
    // Base amount = compute_units * base_rate
    let base_amount = compute_units.saturating_mul(params.base_rate);

    // Apply tier multiplier
    let tier_multiplier = tier.multiplier() as u128;
    let with_tier = base_amount.saturating_mul(tier_multiplier) / 100;
    let tier_bonus = with_tier.saturating_sub(base_amount);

    // Performance bonus
    let performance_bonus = with_tier
        .saturating_mul(params.performance_bonus_pct as u128)
        / 100;

    // Total
    let total = with_tier
        .saturating_add(performance_bonus)
        .saturating_add(params.loyalty_bonus);

    CompensationResult {
        base_amount,
        tier_bonus,
        performance_bonus,
        loyalty_bonus: params.loyalty_bonus,
        total,
        meets_minimum: total >= params.min_payout,
    }
}

/// Batch calculate compensation for multiple contributors
pub fn calculate_batch(
    contributions: &[(u128, CompensationTier)],
    params: &CompensationParams,
) -> Vec<CompensationResult> {
    contributions
        .iter()
        .map(|(units, tier)| calculate_compensation(*units, *tier, params))
        .collect()
}

/// Calculate total payout for a batch
pub fn total_payout(results: &[CompensationResult]) -> u128 {
    results
        .iter()
        .filter(|r| r.meets_minimum)
        .map(|r| r.total)
        .fold(0u128, |acc, x| acc.saturating_add(x))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bronze_tier_compensation() {
        let params = CompensationParams::default();
        let result = calculate_compensation(10, CompensationTier::Bronze, &params);
        assert_eq!(result.base_amount, 10_000_000_000_000);
        assert_eq!(result.tier_bonus, 0);
        assert_eq!(result.total, 10_000_000_000_000);
        assert!(result.meets_minimum);
    }

    #[test]
    fn test_gold_tier_compensation() {
        let params = CompensationParams::default();
        let result = calculate_compensation(10, CompensationTier::Gold, &params);
        // 10 * 1e12 = 10e12 base
        // 10e12 * 150 / 100 = 15e12 with tier
        assert_eq!(result.total, 15_000_000_000_000);
        assert_eq!(result.tier_bonus, 5_000_000_000_000);
    }

    #[test]
    fn test_performance_bonus() {
        let params = CompensationParams {
            performance_bonus_pct: 10,
            ..Default::default()
        };
        let result = calculate_compensation(10, CompensationTier::Bronze, &params);
        // base: 10e12, performance: 10e12 * 10% = 1e12
        assert_eq!(result.performance_bonus, 1_000_000_000_000);
        assert_eq!(result.total, 11_000_000_000_000);
    }

    #[test]
    fn test_below_minimum() {
        let params = CompensationParams {
            min_payout: 100_000_000_000_000, // 100 units
            ..Default::default()
        };
        let result = calculate_compensation(1, CompensationTier::Bronze, &params);
        assert!(!result.meets_minimum);
    }

    #[test]
    fn test_batch_calculation() {
        let params = CompensationParams::default();
        let contributions = vec![
            (10, CompensationTier::Bronze),
            (20, CompensationTier::Silver),
            (30, CompensationTier::Gold),
        ];
        let results = calculate_batch(&contributions, &params);
        assert_eq!(results.len(), 3);
        
        let total = total_payout(&results);
        // Bronze: 10e12, Silver: 25e12, Gold: 45e12 = 80e12
        assert_eq!(total, 80_000_000_000_000);
    }
}
