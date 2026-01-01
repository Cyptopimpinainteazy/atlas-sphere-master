//! Mutation operators for X3 bytecode evolution

use crate::chromosome::{Chromosome, Gene, GeneType};
use crate::error::Result;
use rand::Rng;
use rand_chacha::ChaCha20Rng;

/// Trait for mutation operators
pub trait MutationOperator: Send + Sync {
    /// Apply mutation to a chromosome
    fn mutate(&self, chromosome: &mut Chromosome, rng: &mut ChaCha20Rng) -> Result<bool>;

    /// Get mutation type name
    fn name(&self) -> &'static str;
}

/// Configuration for mutation operations
#[derive(Debug, Clone)]
pub struct MutationConfig {
    /// Probability of mutating each gene
    pub gene_mutation_rate: f64,
    /// Maximum percentage change for numeric values
    pub max_delta_percent: f64,
    /// Whether to allow structural mutations
    pub allow_structural: bool,
    /// Maximum genes to mutate per operation
    pub max_mutations: usize,
}

impl Default for MutationConfig {
    fn default() -> Self {
        Self {
            gene_mutation_rate: 0.1,
            max_delta_percent: 0.2,
            allow_structural: false,
            max_mutations: 5,
        }
    }
}

/// Parameter mutation - adjusts numeric values
pub struct ParameterMutation {
    config: MutationConfig,
}

impl ParameterMutation {
    pub fn new(mutation_rate: f64) -> Self {
        Self {
            config: MutationConfig {
                gene_mutation_rate: mutation_rate,
                ..Default::default()
            },
        }
    }

    pub fn with_config(config: MutationConfig) -> Self {
        Self { config }
    }
}

impl MutationOperator for ParameterMutation {
    fn mutate(&self, chromosome: &mut Chromosome, rng: &mut ChaCha20Rng) -> Result<bool> {
        let mut mutated = false;
        let mut mutation_count = 0;

        for gene in chromosome.genes_mut() {
            if mutation_count >= self.config.max_mutations {
                break;
            }

            if !gene.is_mutable() {
                continue;
            }

            if gene.gene_type != GeneType::Parameter {
                continue;
            }

            if rng.gen::<f64>() > self.config.gene_mutation_rate {
                continue;
            }

            // Calculate mutation delta
            let current = gene.value as f64;
            let range = gene.constraints.max.saturating_sub(gene.constraints.min) as f64;
            let max_delta = range * self.config.max_delta_percent;

            // Apply gaussian-like mutation
            let delta = (rng.gen::<f64>() - 0.5) * 2.0 * max_delta;
            let new_value = (current + delta)
                .max(gene.constraints.min as f64)
                .min(gene.constraints.max as f64) as u64;

            if gene.mutate(new_value) {
                mutated = true;
                mutation_count += 1;
            }
        }

        Ok(mutated)
    }

    fn name(&self) -> &'static str {
        "ParameterMutation"
    }
}

/// Logic mutation - changes opcodes and control flow
pub struct LogicMutation {
    config: MutationConfig,
    /// Opcode substitution map (original -> alternatives)
    substitutions: Vec<(u64, Vec<u64>)>,
}

impl LogicMutation {
    pub fn new(mutation_rate: f64) -> Self {
        Self {
            config: MutationConfig {
                gene_mutation_rate: mutation_rate,
                allow_structural: true,
                ..Default::default()
            },
            substitutions: Self::default_substitutions(),
        }
    }

    fn default_substitutions() -> Vec<(u64, Vec<u64>)> {
        vec![
            // Arithmetic substitutions
            (0x20, vec![0x21, 0x22, 0x23]), // ADD -> SUB, MUL, DIV
            (0x21, vec![0x20, 0x22, 0x23]), // SUB -> ADD, MUL, DIV
            // Comparison substitutions
            (0x30, vec![0x31, 0x32, 0x33, 0x34, 0x35]), // LT -> GT, LE, GE, EQ, NE
            (0x31, vec![0x30, 0x32, 0x33, 0x34, 0x35]), // GT -> LT, LE, GE, EQ, NE
            // Branch substitutions
            (0x10, vec![0x11, 0x12]), // JZ -> JNZ, JMP
            (0x11, vec![0x10, 0x12]), // JNZ -> JZ, JMP
        ]
    }
}

impl MutationOperator for LogicMutation {
    fn mutate(&self, chromosome: &mut Chromosome, rng: &mut ChaCha20Rng) -> Result<bool> {
        if !self.config.allow_structural {
            return Ok(false);
        }

        let mut mutated = false;
        let mut mutation_count = 0;

        for gene in chromosome.genes_mut() {
            if mutation_count >= self.config.max_mutations {
                break;
            }

            if !gene.is_mutable() {
                continue;
            }

            if gene.gene_type != GeneType::Opcode && gene.gene_type != GeneType::ControlFlow {
                continue;
            }

            if rng.gen::<f64>() > self.config.gene_mutation_rate {
                continue;
            }

            // Find substitution
            let opcode = gene.value & 0xFF;
            for (orig, alts) in &self.substitutions {
                if opcode == *orig && !alts.is_empty() {
                    let new_opcode = alts[rng.gen_range(0..alts.len())];
                    let new_value = (gene.value & !0xFF) | new_opcode;
                    gene.value = new_value;
                    mutated = true;
                    mutation_count += 1;
                    break;
                }
            }
        }

        Ok(mutated)
    }

    fn name(&self) -> &'static str {
        "LogicMutation"
    }
}

/// Gaussian mutation - applies gaussian noise to parameters
pub struct GaussianMutation {
    /// Standard deviation relative to value range
    sigma: f64,
    mutation_rate: f64,
}

impl GaussianMutation {
    pub fn new(sigma: f64, mutation_rate: f64) -> Self {
        Self {
            sigma,
            mutation_rate,
        }
    }
}

impl MutationOperator for GaussianMutation {
    fn mutate(&self, chromosome: &mut Chromosome, rng: &mut ChaCha20Rng) -> Result<bool> {
        let mut mutated = false;

        for gene in chromosome.genes_mut() {
            if !gene.is_mutable() || gene.gene_type != GeneType::Parameter {
                continue;
            }

            if rng.gen::<f64>() > self.mutation_rate {
                continue;
            }

            // Box-Muller transform for gaussian
            let u1: f64 = rng.gen();
            let u2: f64 = rng.gen();
            let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();

            let range = (gene.constraints.max - gene.constraints.min) as f64;
            let delta = z * self.sigma * range;

            let new_value = (gene.value as f64 + delta)
                .max(gene.constraints.min as f64)
                .min(gene.constraints.max as f64) as u64;

            if gene.mutate(new_value) {
                mutated = true;
            }
        }

        Ok(mutated)
    }

    fn name(&self) -> &'static str {
        "GaussianMutation"
    }
}

/// Swap mutation - swaps positions of genes
pub struct SwapMutation {
    swap_rate: f64,
}

impl SwapMutation {
    pub fn new(swap_rate: f64) -> Self {
        Self { swap_rate }
    }
}

impl MutationOperator for SwapMutation {
    fn mutate(&self, chromosome: &mut Chromosome, rng: &mut ChaCha20Rng) -> Result<bool> {
        if chromosome.len() < 2 {
            return Ok(false);
        }

        if rng.gen::<f64>() > self.swap_rate {
            return Ok(false);
        }

        // Find two mutable genes of same type
        let mutable: Vec<(usize, GeneType)> = chromosome
            .genes()
            .iter()
            .enumerate()
            .filter(|(_, g)| g.is_mutable())
            .map(|(i, g)| (i, g.gene_type))
            .collect();

        if mutable.len() < 2 {
            return Ok(false);
        }

        // Try to find two genes of same type
        let idx1 = rng.gen_range(0..mutable.len());
        let (i1, type1) = mutable[idx1];

        let same_type: Vec<usize> = mutable
            .iter()
            .filter(|(i, t)| *i != i1 && *t == type1)
            .map(|(i, _)| *i)
            .collect();

        if same_type.is_empty() {
            return Ok(false);
        }

        let i2 = same_type[rng.gen_range(0..same_type.len())];

        // Swap values
        let genes = chromosome.genes_mut();
        let v1 = genes[i1].value;
        let v2 = genes[i2].value;
        genes[i1].value = v2;
        genes[i2].value = v1;

        Ok(true)
    }

    fn name(&self) -> &'static str {
        "SwapMutation"
    }
}

/// Composite mutation - applies multiple mutation operators
pub struct CompositeMutation {
    operators: Vec<Box<dyn MutationOperator>>,
}

impl CompositeMutation {
    pub fn new() -> Self {
        Self {
            operators: Vec::new(),
        }
    }

    pub fn add<M: MutationOperator + 'static>(mut self, op: M) -> Self {
        self.operators.push(Box::new(op));
        self
    }

    pub fn default_operators(mutation_rate: f64) -> Self {
        Self::new()
            .add(ParameterMutation::new(mutation_rate))
            .add(LogicMutation::new(mutation_rate * 0.5))
            .add(GaussianMutation::new(0.1, mutation_rate * 0.3))
            .add(SwapMutation::new(mutation_rate * 0.1))
    }
}

impl Default for CompositeMutation {
    fn default() -> Self {
        Self::default_operators(0.1)
    }
}

impl MutationOperator for CompositeMutation {
    fn mutate(&self, chromosome: &mut Chromosome, rng: &mut ChaCha20Rng) -> Result<bool> {
        let mut any_mutated = false;

        for op in &self.operators {
            if op.mutate(chromosome, rng)? {
                any_mutated = true;
            }
        }

        Ok(any_mutated)
    }

    fn name(&self) -> &'static str {
        "CompositeMutation"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn test_chromosome() -> Chromosome {
        // Create test bytecode with parameters
        let bytecode = vec![
            0x20, 0x64, 0x00, 0x00, // ADD with param 100
            0x20, 0xC8, 0x00, 0x00, // ADD with param 200
            0x30, 0x0A, 0x00, 0x00, // CMP with param 10
        ];
        Chromosome::from_bytecode(bytecode).unwrap()
    }

    #[test]
    fn test_parameter_mutation() {
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        let mut chromosome = test_chromosome();
        let mutation = ParameterMutation::new(1.0); // 100% rate for testing

        let original = chromosome.to_bytecode();
        mutation.mutate(&mut chromosome, &mut rng).unwrap();
        let mutated = chromosome.to_bytecode();

        // Should be different (with high probability)
        // Note: might still be same in rare cases
        assert!(original != mutated || chromosome.len() == 0);
    }

    #[test]
    fn test_gaussian_mutation() {
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        let mut chromosome = test_chromosome();
        let mutation = GaussianMutation::new(0.1, 1.0);

        let result = mutation.mutate(&mut chromosome, &mut rng).unwrap();
        assert!(result || chromosome.genes().iter().all(|g| !g.is_mutable()));
    }

    #[test]
    fn test_composite_mutation() {
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        let mut chromosome = test_chromosome();
        let mutation = CompositeMutation::default_operators(0.5);

        // Run multiple times to ensure at least one mutation occurs
        let mut mutated = false;
        for _ in 0..10 {
            if mutation.mutate(&mut chromosome, &mut rng).unwrap() {
                mutated = true;
                break;
            }
        }
        assert!(mutated);
    }
}
