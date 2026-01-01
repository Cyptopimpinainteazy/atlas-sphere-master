//! Mutation operators for evolutionary strategies
//!
//! Implements various mutation strategies:
//! - Uniform mutation
//! - Gaussian mutation
//! - Adaptive mutation (fitness-based)
//! - Polynomial bounded mutation (SBX-style)

use serde::{Deserialize, Serialize};

/// Mutation operator type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MutationOperator {
    /// Uniform random mutation
    Uniform {
        /// Mutation strength (0.0 - 1.0)
        strength: f64,
    },
    /// Gaussian perturbation
    Gaussian {
        /// Standard deviation
        sigma: f64,
    },
    /// Adaptive mutation based on fitness
    Adaptive,
    /// Polynomial bounded mutation (like SBX)
    PolynomialBounded {
        /// Distribution index (higher = smaller changes)
        eta: f64,
    },
}

impl Default for MutationOperator {
    fn default() -> Self {
        Self::Gaussian { sigma: 0.1 }
    }
}

impl MutationOperator {
    /// Create uniform mutation operator
    pub fn uniform(strength: f64) -> Self {
        Self::Uniform {
            strength: strength.clamp(0.0, 1.0),
        }
    }

    /// Create Gaussian mutation operator
    pub fn gaussian(sigma: f64) -> Self {
        Self::Gaussian { sigma: sigma.abs() }
    }

    /// Create adaptive mutation operator
    pub fn adaptive() -> Self {
        Self::Adaptive
    }

    /// Create polynomial bounded mutation operator
    pub fn polynomial(eta: f64) -> Self {
        Self::PolynomialBounded { eta: eta.abs() }
    }

    /// Get effective mutation strength
    pub fn strength(&self) -> f64 {
        match self {
            Self::Uniform { strength } => *strength,
            Self::Gaussian { sigma } => *sigma,
            Self::Adaptive => 0.3, // Default for adaptive
            Self::PolynomialBounded { eta } => 1.0 / (*eta + 1.0),
        }
    }
}

/// Mutation statistics
#[derive(Debug, Default, Clone)]
pub struct Mutation {
    /// Total mutations applied
    pub total_mutations: usize,
    /// Beneficial mutations (improved fitness)
    pub beneficial: usize,
    /// Neutral mutations
    pub neutral: usize,
    /// Deleterious mutations
    pub deleterious: usize,
    /// Average mutation magnitude
    pub avg_magnitude: f64,
}

impl Mutation {
    /// Create new mutation tracker
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a mutation result
    pub fn record(&mut self, fitness_delta: f64, magnitude: f64) {
        self.total_mutations += 1;

        if fitness_delta > 0.001 {
            self.beneficial += 1;
        } else if fitness_delta < -0.001 {
            self.deleterious += 1;
        } else {
            self.neutral += 1;
        }

        // Running average
        let n = self.total_mutations as f64;
        self.avg_magnitude = self.avg_magnitude * ((n - 1.0) / n) + magnitude / n;
    }

    /// Get beneficial mutation rate
    pub fn beneficial_rate(&self) -> f64 {
        if self.total_mutations > 0 {
            self.beneficial as f64 / self.total_mutations as f64
        } else {
            0.0
        }
    }

    /// Reset statistics
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mutation_operators() {
        let uniform = MutationOperator::uniform(0.5);
        assert!((uniform.strength() - 0.5).abs() < 1e-10);

        let gaussian = MutationOperator::gaussian(0.1);
        assert!((gaussian.strength() - 0.1).abs() < 1e-10);

        let adaptive = MutationOperator::adaptive();
        assert!((adaptive.strength() - 0.3).abs() < 1e-10);

        let poly = MutationOperator::polynomial(20.0);
        assert!((poly.strength() - 1.0 / 21.0).abs() < 1e-10);
    }

    #[test]
    fn test_mutation_tracking() {
        let mut tracker = Mutation::new();

        tracker.record(0.1, 0.05); // Beneficial
        tracker.record(-0.2, 0.08); // Deleterious
        tracker.record(0.0, 0.01); // Neutral

        assert_eq!(tracker.total_mutations, 3);
        assert_eq!(tracker.beneficial, 1);
        assert_eq!(tracker.deleterious, 1);
        assert_eq!(tracker.neutral, 1);
        assert!((tracker.beneficial_rate() - 1.0 / 3.0).abs() < 1e-10);
    }
}
