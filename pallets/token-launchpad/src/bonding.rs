//! Bonding curve implementations

use crate::types::BondingCurveType;

/// Compute token price at given supply
pub fn compute_price(curve_type: BondingCurveType, supply: u128) -> u128 {
    match curve_type {
        BondingCurveType::Linear => linear_price(supply),
        BondingCurveType::Exponential => exponential_price(supply),
        BondingCurveType::Sigmoid => sigmoid_price(supply),
    }
}

/// Linear bonding curve: price = 1 + (supply / 1000)
fn linear_price(supply: u128) -> u128 {
    1_000_000 + (supply / 1000)
}

/// Exponential bonding curve: price ≈ 1 * e^(supply / 10_000_000)
/// Simplified: price = 1 + (supply^2 / 10_000_000_000)
fn exponential_price(supply: u128) -> u128 {
    1_000_000 + (supply.saturating_mul(supply) / 10_000_000_000)
}

/// Sigmoid bonding curve: smooth S-curve transition
fn sigmoid_price(supply: u128) -> u128 {
    // Simplified sigmoid: price = max / (1 + e^(-k*(supply - midpoint)))
    // For MVP: linear approximation
    linear_price(supply)
}

/// Compute tokens for given amount
pub fn compute_tokens(curve_type: BondingCurveType, amount: u128) -> u128 {
    // Simple approximation: tokens ≈ amount / avg_price
    let avg_price = compute_price(curve_type, 1_000_000);
    amount / avg_price
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_price() {
        let p1 = linear_price(0);
        let p2 = linear_price(1000);
        assert!(p2 > p1);
    }

    #[test]
    fn test_exponential_price() {
        let p1 = exponential_price(0);
        let p2 = exponential_price(1000);
        assert!(p2 > p1);
    }

    #[test]
    fn test_compute_tokens() {
        let tokens = compute_tokens(BondingCurveType::Linear, 1_000_000);
        assert!(tokens > 0);
    }
}
