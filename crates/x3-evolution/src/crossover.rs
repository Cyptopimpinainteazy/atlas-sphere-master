//! Crossover operators for genetic recombination

use crate::chromosome::Chromosome;
use crate::error::Result;
use rand::Rng;
use rand_chacha::ChaCha20Rng;

/// Trait for crossover operators
pub trait CrossoverOperator: Send + Sync {
    /// Perform crossover between two parent chromosomes
    fn crossover(
        &self,
        parent1: &Chromosome,
        parent2: &Chromosome,
        rng: &mut ChaCha20Rng,
    ) -> Result<Chromosome>;

    /// Get crossover operator name
    fn name(&self) -> &'static str;
}

/// Uniform crossover - each gene randomly chosen from either parent
pub struct UniformCrossover {
    /// Probability of taking gene from parent1
    bias: f64,
}

impl UniformCrossover {
    pub fn new(bias: f64) -> Self {
        Self {
            bias: bias.clamp(0.0, 1.0),
        }
    }

    pub fn unbiased() -> Self {
        Self::new(0.5)
    }
}

impl CrossoverOperator for UniformCrossover {
    fn crossover(
        &self,
        parent1: &Chromosome,
        parent2: &Chromosome,
        rng: &mut ChaCha20Rng,
    ) -> Result<Chromosome> {
        // Use parent1 as base template
        let mut child = parent1.clone();

        // For each gene position, randomly choose from parent
        let min_len = parent1.len().min(parent2.len());

        for i in 0..min_len {
            if rng.gen::<f64>() > self.bias {
                // Take from parent2
                if let (Some(child_gene), Some(parent2_gene)) = (child.get_mut(i), parent2.get(i)) {
                    if child_gene.gene_type == parent2_gene.gene_type {
                        child_gene.value = parent2_gene.value;
                    }
                }
            }
        }

        // Update metadata
        child.metadata.parents = vec![parent1.metadata.id, parent2.metadata.id];
        child.metadata.id = rand::random();

        Ok(child)
    }

    fn name(&self) -> &'static str {
        "UniformCrossover"
    }
}

/// Single-point crossover - split at random point
pub struct SinglePointCrossover;

impl SinglePointCrossover {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SinglePointCrossover {
    fn default() -> Self {
        Self::new()
    }
}

impl CrossoverOperator for SinglePointCrossover {
    fn crossover(
        &self,
        parent1: &Chromosome,
        parent2: &Chromosome,
        rng: &mut ChaCha20Rng,
    ) -> Result<Chromosome> {
        let mut child = parent1.clone();
        let min_len = parent1.len().min(parent2.len());

        if min_len < 2 {
            return Ok(child);
        }

        // Choose crossover point
        let point = rng.gen_range(1..min_len);

        // Take genes after point from parent2
        for i in point..min_len {
            if let (Some(child_gene), Some(parent2_gene)) = (child.get_mut(i), parent2.get(i)) {
                if child_gene.gene_type == parent2_gene.gene_type {
                    child_gene.value = parent2_gene.value;
                }
            }
        }

        child.metadata.parents = vec![parent1.metadata.id, parent2.metadata.id];
        child.metadata.id = rand::random();

        Ok(child)
    }

    fn name(&self) -> &'static str {
        "SinglePointCrossover"
    }
}

/// Two-point crossover - exchange segment between two points
pub struct TwoPointCrossover;

impl TwoPointCrossover {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TwoPointCrossover {
    fn default() -> Self {
        Self::new()
    }
}

impl CrossoverOperator for TwoPointCrossover {
    fn crossover(
        &self,
        parent1: &Chromosome,
        parent2: &Chromosome,
        rng: &mut ChaCha20Rng,
    ) -> Result<Chromosome> {
        let mut child = parent1.clone();
        let min_len = parent1.len().min(parent2.len());

        if min_len < 3 {
            return Ok(child);
        }

        // Choose two crossover points
        let point1 = rng.gen_range(0..min_len - 1);
        let point2 = rng.gen_range(point1 + 1..min_len);

        // Take genes between points from parent2
        for i in point1..point2 {
            if let (Some(child_gene), Some(parent2_gene)) = (child.get_mut(i), parent2.get(i)) {
                if child_gene.gene_type == parent2_gene.gene_type {
                    child_gene.value = parent2_gene.value;
                }
            }
        }

        child.metadata.parents = vec![parent1.metadata.id, parent2.metadata.id];
        child.metadata.id = rand::random();

        Ok(child)
    }

    fn name(&self) -> &'static str {
        "TwoPointCrossover"
    }
}

/// Arithmetic crossover - blend numeric values
pub struct ArithmeticCrossover {
    /// Blending factor (0.0 = parent1, 1.0 = parent2)
    alpha: f64,
}

impl ArithmeticCrossover {
    pub fn new(alpha: f64) -> Self {
        Self {
            alpha: alpha.clamp(0.0, 1.0),
        }
    }

    /// Random blending factor for each gene
    pub fn random() -> Self {
        Self::new(0.5) // Will be randomized per gene
    }
}

impl CrossoverOperator for ArithmeticCrossover {
    fn crossover(
        &self,
        parent1: &Chromosome,
        parent2: &Chromosome,
        rng: &mut ChaCha20Rng,
    ) -> Result<Chromosome> {
        let mut child = parent1.clone();
        let min_len = parent1.len().min(parent2.len());

        for i in 0..min_len {
            if let (Some(child_gene), Some(parent2_gene)) = (child.get_mut(i), parent2.get(i)) {
                // Only blend numeric types
                if child_gene.gene_type != parent2_gene.gene_type {
                    continue;
                }

                // Use random alpha for variety
                let alpha = if self.alpha == 0.5 {
                    rng.gen::<f64>()
                } else {
                    self.alpha
                };

                // Arithmetic blend
                let v1 = child_gene.value as f64;
                let v2 = parent2_gene.value as f64;
                let blended = v1 * (1.0 - alpha) + v2 * alpha;

                let new_value = blended
                    .max(child_gene.constraints.min as f64)
                    .min(child_gene.constraints.max as f64) as u64;

                child_gene.value = new_value;
            }
        }

        child.metadata.parents = vec![parent1.metadata.id, parent2.metadata.id];
        child.metadata.id = rand::random();

        Ok(child)
    }

    fn name(&self) -> &'static str {
        "ArithmeticCrossover"
    }
}

/// Adaptive crossover - chooses operator based on fitness
pub struct AdaptiveCrossover {
    operators: Vec<(Box<dyn CrossoverOperator>, f64)>, // (operator, success_rate)
}

impl AdaptiveCrossover {
    pub fn new() -> Self {
        Self {
            operators: vec![
                (Box::new(UniformCrossover::unbiased()), 1.0),
                (Box::new(SinglePointCrossover::new()), 1.0),
                (Box::new(TwoPointCrossover::new()), 1.0),
                (Box::new(ArithmeticCrossover::random()), 1.0),
            ],
        }
    }
}

impl Default for AdaptiveCrossover {
    fn default() -> Self {
        Self::new()
    }
}

impl CrossoverOperator for AdaptiveCrossover {
    fn crossover(
        &self,
        parent1: &Chromosome,
        parent2: &Chromosome,
        rng: &mut ChaCha20Rng,
    ) -> Result<Chromosome> {
        // Roulette selection based on success rates
        let total: f64 = self.operators.iter().map(|(_, r)| r).sum();
        let mut choice = rng.gen::<f64>() * total;

        for (op, rate) in &self.operators {
            choice -= rate;
            if choice <= 0.0 {
                return op.crossover(parent1, parent2, rng);
            }
        }

        // Fallback to uniform
        UniformCrossover::unbiased().crossover(parent1, parent2, rng)
    }

    fn name(&self) -> &'static str {
        "AdaptiveCrossover"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn test_parents() -> (Chromosome, Chromosome) {
        let bytecode1 = vec![0x20, 0x64, 0x00, 0x00, 0x20, 0xC8, 0x00, 0x00];
        let bytecode2 = vec![0x20, 0x32, 0x00, 0x00, 0x20, 0xFA, 0x00, 0x00];

        (
            Chromosome::from_bytecode(bytecode1).unwrap(),
            Chromosome::from_bytecode(bytecode2).unwrap(),
        )
    }

    #[test]
    fn test_uniform_crossover() {
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        let (p1, p2) = test_parents();
        let crossover = UniformCrossover::unbiased();

        let child = crossover.crossover(&p1, &p2, &mut rng).unwrap();
        assert!(!child.is_empty());
        assert_eq!(child.metadata.parents.len(), 2);
    }

    #[test]
    fn test_single_point_crossover() {
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        let (p1, p2) = test_parents();
        let crossover = SinglePointCrossover::new();

        let child = crossover.crossover(&p1, &p2, &mut rng).unwrap();
        assert!(!child.is_empty());
    }

    #[test]
    fn test_two_point_crossover() {
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        let (p1, p2) = test_parents();
        let crossover = TwoPointCrossover::new();

        let child = crossover.crossover(&p1, &p2, &mut rng).unwrap();
        assert!(!child.is_empty());
    }

    #[test]
    fn test_arithmetic_crossover() {
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        let (p1, p2) = test_parents();
        let crossover = ArithmeticCrossover::new(0.5);

        let child = crossover.crossover(&p1, &p2, &mut rng).unwrap();
        assert!(!child.is_empty());
    }

    #[test]
    fn test_adaptive_crossover() {
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        let (p1, p2) = test_parents();
        let crossover = AdaptiveCrossover::new();

        let child = crossover.crossover(&p1, &p2, &mut rng).unwrap();
        assert!(!child.is_empty());
    }
}
