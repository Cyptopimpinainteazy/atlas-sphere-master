//! Genome representation for evolutionary strategies
//!
//! Supports multiple gene types for different optimization problems:
//! - Float genes for continuous parameters
//! - Integer genes for discrete choices
//! - Boolean genes for feature flags
//! - Categorical genes for architecture search

use super::mutation::MutationOperator;
use crate::types::StrategyId;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single gene in the genome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gene {
    /// Gene identifier
    pub name: String,
    /// Gene type and value
    pub gene_type: GeneType,
    /// Is this gene mutable
    pub mutable: bool,
}

/// Gene type variants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GeneType {
    /// Continuous float value with bounds
    Float { value: f64, min: f64, max: f64 },
    /// Discrete integer value with bounds
    Integer { value: i64, min: i64, max: i64 },
    /// Boolean flag
    Boolean { value: bool },
    /// Categorical selection
    Categorical { value: usize, options: Vec<String> },
    /// Permutation (for routing problems)
    Permutation { values: Vec<usize> },
    /// Binary string (for QUBO problems)
    Binary { bits: Vec<bool> },
}

impl Gene {
    /// Create a float gene
    pub fn float(name: &str, value: f64, min: f64, max: f64) -> Self {
        Self {
            name: name.to_string(),
            gene_type: GeneType::Float { value, min, max },
            mutable: true,
        }
    }

    /// Create an integer gene
    pub fn integer(name: &str, value: i64, min: i64, max: i64) -> Self {
        Self {
            name: name.to_string(),
            gene_type: GeneType::Integer { value, min, max },
            mutable: true,
        }
    }

    /// Create a boolean gene
    pub fn boolean(name: &str, value: bool) -> Self {
        Self {
            name: name.to_string(),
            gene_type: GeneType::Boolean { value },
            mutable: true,
        }
    }

    /// Create a categorical gene
    pub fn categorical(name: &str, value: usize, options: Vec<String>) -> Self {
        Self {
            name: name.to_string(),
            gene_type: GeneType::Categorical { value, options },
            mutable: true,
        }
    }

    /// Create a permutation gene
    pub fn permutation(name: &str, size: usize) -> Self {
        let values: Vec<usize> = (0..size).collect();
        Self {
            name: name.to_string(),
            gene_type: GeneType::Permutation { values },
            mutable: true,
        }
    }

    /// Create a binary gene
    pub fn binary(name: &str, size: usize) -> Self {
        Self {
            name: name.to_string(),
            gene_type: GeneType::Binary {
                bits: vec![false; size],
            },
            mutable: true,
        }
    }

    /// Get float value
    pub fn as_float(&self) -> Option<f64> {
        match &self.gene_type {
            GeneType::Float { value, .. } => Some(*value),
            _ => None,
        }
    }

    /// Get integer value
    pub fn as_integer(&self) -> Option<i64> {
        match &self.gene_type {
            GeneType::Integer { value, .. } => Some(*value),
            _ => None,
        }
    }

    /// Get boolean value
    pub fn as_boolean(&self) -> Option<bool> {
        match &self.gene_type {
            GeneType::Boolean { value } => Some(*value),
            _ => None,
        }
    }

    /// Randomize this gene
    pub fn randomize(&mut self) {
        let mut rng = thread_rng();
        match &mut self.gene_type {
            GeneType::Float { value, min, max } => {
                *value = rng.gen_range(*min..=*max);
            }
            GeneType::Integer { value, min, max } => {
                *value = rng.gen_range(*min..=*max);
            }
            GeneType::Boolean { value } => {
                *value = rng.gen_bool(0.5);
            }
            GeneType::Categorical { value, options } => {
                *value = rng.gen_range(0..options.len());
            }
            GeneType::Permutation { values } => {
                // Fisher-Yates shuffle
                let n = values.len();
                for i in (1..n).rev() {
                    let j = rng.gen_range(0..=i);
                    values.swap(i, j);
                }
            }
            GeneType::Binary { bits } => {
                for bit in bits.iter_mut() {
                    *bit = rng.gen_bool(0.5);
                }
            }
        }
    }

    /// Mutate this gene with given strength
    pub fn mutate(&mut self, strength: f64) {
        if !self.mutable {
            return;
        }

        let mut rng = thread_rng();
        match &mut self.gene_type {
            GeneType::Float { value, min, max } => {
                // Gaussian mutation
                let range = *max - *min;
                let delta = rng.gen::<f64>() * 2.0 - 1.0; // -1 to 1
                *value += delta * range * strength;
                *value = value.clamp(*min, *max);
            }
            GeneType::Integer { value, min, max } => {
                // Step mutation
                let range = (*max - *min) as f64;
                let step = (rng.gen::<f64>() * 2.0 - 1.0) * range * strength;
                *value = (*value + step as i64).clamp(*min, *max);
            }
            GeneType::Boolean { value } => {
                // Flip with probability proportional to strength
                if rng.gen::<f64>() < strength {
                    *value = !*value;
                }
            }
            GeneType::Categorical { value, options } => {
                // Random change with probability proportional to strength
                if rng.gen::<f64>() < strength {
                    *value = rng.gen_range(0..options.len());
                }
            }
            GeneType::Permutation { values } => {
                // Swap mutation
                let n = values.len();
                let num_swaps = ((n as f64 * strength) as usize).max(1);
                for _ in 0..num_swaps {
                    let i = rng.gen_range(0..n);
                    let j = rng.gen_range(0..n);
                    values.swap(i, j);
                }
            }
            GeneType::Binary { bits } => {
                // Bit flip mutation
                for bit in bits.iter_mut() {
                    if rng.gen::<f64>() < strength {
                        *bit = !*bit;
                    }
                }
            }
        }
    }
}

/// Complete genome representing a strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Genome {
    /// Unique identifier
    pub id: StrategyId,
    /// Genes
    pub genes: Vec<Gene>,
    /// Fitness score (updated externally)
    pub fitness: f64,
    /// Generation this genome was created
    pub birth_generation: u64,
    /// Parent IDs (for lineage tracking)
    pub parents: Vec<StrategyId>,
    /// Species ID (for speciation)
    pub species_id: Option<usize>,
    /// Metadata
    pub metadata: HashMap<String, String>,
}

impl Genome {
    /// Create new empty genome
    pub fn new() -> Self {
        Self {
            id: StrategyId::new_v4(),
            genes: Vec::new(),
            fitness: 0.0,
            birth_generation: 0,
            parents: Vec::new(),
            species_id: None,
            metadata: HashMap::new(),
        }
    }

    /// Create genome with float genes
    pub fn new_float(num_genes: usize, min: f64, max: f64) -> Self {
        let mut genome = Self::new();
        for i in 0..num_genes {
            let mut gene = Gene::float(&format!("gene_{}", i), (min + max) / 2.0, min, max);
            gene.randomize();
            genome.genes.push(gene);
        }
        genome
    }

    /// Create genome with binary genes
    pub fn new_binary(num_bits: usize) -> Self {
        let mut genome = Self::new();
        let mut gene = Gene::binary("bits", num_bits);
        gene.randomize();
        genome.genes.push(gene);
        genome
    }

    /// Create genome with permutation gene
    pub fn new_permutation(size: usize) -> Self {
        let mut genome = Self::new();
        let mut gene = Gene::permutation("route", size);
        gene.randomize();
        genome.genes.push(gene);
        genome
    }

    /// Add a gene
    pub fn add_gene(&mut self, gene: Gene) {
        self.genes.push(gene);
    }

    /// Get gene by name
    pub fn get_gene(&self, name: &str) -> Option<&Gene> {
        self.genes.iter().find(|g| g.name == name)
    }

    /// Get mutable gene by name
    pub fn get_gene_mut(&mut self, name: &str) -> Option<&mut Gene> {
        self.genes.iter_mut().find(|g| g.name == name)
    }

    /// Randomize all genes
    pub fn randomize(&mut self) {
        self.id = StrategyId::new_v4();
        for gene in &mut self.genes {
            gene.randomize();
        }
    }

    /// Mutate genome with given operator
    pub fn mutate(&mut self, operator: &MutationOperator) {
        let strength = match operator {
            MutationOperator::Uniform { strength } => *strength,
            MutationOperator::Gaussian { sigma } => *sigma,
            MutationOperator::Adaptive => {
                // Higher mutation for lower fitness
                (1.0 - self.fitness.clamp(0.0, 1.0)) * 0.5 + 0.1
            }
            MutationOperator::PolynomialBounded { eta } => {
                // Use distribution index for strength
                1.0 / (*eta + 1.0)
            }
        };

        // Create new ID for mutated genome
        self.id = StrategyId::new_v4();

        for gene in &mut self.genes {
            if gene.mutable && rand::random::<f64>() < 0.5 {
                gene.mutate(strength);
            }
        }
    }

    /// Crossover with another genome
    pub fn crossover(&self, other: &Genome) -> Genome {
        let mut child = self.clone();
        child.id = StrategyId::new_v4();
        child.parents = vec![self.id, other.id];
        child.fitness = 0.0;

        // Uniform crossover
        for (i, gene) in child.genes.iter_mut().enumerate() {
            if rand::random::<bool>() && i < other.genes.len() {
                *gene = other.genes[i].clone();
            }
        }

        child
    }

    /// Calculate genetic distance to another genome
    pub fn distance(&self, other: &Genome) -> f64 {
        let mut total_diff = 0.0;
        let mut count = 0;

        for (g1, g2) in self.genes.iter().zip(other.genes.iter()) {
            let diff = match (&g1.gene_type, &g2.gene_type) {
                (
                    GeneType::Float {
                        value: v1,
                        min,
                        max,
                    },
                    GeneType::Float { value: v2, .. },
                ) => {
                    let range = max - min;
                    ((v1 - v2).abs() / range).min(1.0)
                }
                (
                    GeneType::Integer {
                        value: v1,
                        min,
                        max,
                    },
                    GeneType::Integer { value: v2, .. },
                ) => {
                    let range = (max - min) as f64;
                    ((v1 - v2).abs() as f64 / range).min(1.0)
                }
                (GeneType::Boolean { value: v1 }, GeneType::Boolean { value: v2 }) => {
                    if v1 != v2 {
                        1.0
                    } else {
                        0.0
                    }
                }
                (
                    GeneType::Categorical { value: v1, .. },
                    GeneType::Categorical { value: v2, .. },
                ) => {
                    if v1 != v2 {
                        1.0
                    } else {
                        0.0
                    }
                }
                (GeneType::Permutation { values: v1 }, GeneType::Permutation { values: v2 }) => {
                    // Kendall tau distance
                    let mut inversions = 0;
                    for i in 0..v1.len() {
                        for j in (i + 1)..v1.len() {
                            let pos1_i = v1.iter().position(|&x| x == i).unwrap_or(0);
                            let pos1_j = v1.iter().position(|&x| x == j).unwrap_or(0);
                            let pos2_i = v2.iter().position(|&x| x == i).unwrap_or(0);
                            let pos2_j = v2.iter().position(|&x| x == j).unwrap_or(0);
                            if (pos1_i < pos1_j) != (pos2_i < pos2_j) {
                                inversions += 1;
                            }
                        }
                    }
                    let max_inversions = v1.len() * (v1.len() - 1) / 2;
                    inversions as f64 / max_inversions.max(1) as f64
                }
                (GeneType::Binary { bits: b1 }, GeneType::Binary { bits: b2 }) => {
                    // Hamming distance
                    let diffs: usize = b1.iter().zip(b2.iter()).filter(|(a, b)| a != b).count();
                    diffs as f64 / b1.len().max(1) as f64
                }
                _ => 1.0, // Incompatible types
            };
            total_diff += diff;
            count += 1;
        }

        if count > 0 {
            total_diff / count as f64
        } else {
            0.0
        }
    }

    /// Extract parameters as float vector
    pub fn to_float_vec(&self) -> Vec<f64> {
        let mut params = Vec::new();
        for gene in &self.genes {
            match &gene.gene_type {
                GeneType::Float { value, .. } => params.push(*value),
                GeneType::Integer { value, .. } => params.push(*value as f64),
                GeneType::Boolean { value } => params.push(if *value { 1.0 } else { 0.0 }),
                GeneType::Categorical { value, options } => {
                    params.push(*value as f64 / options.len().max(1) as f64)
                }
                _ => {}
            }
        }
        params
    }

    /// Set parameters from float vector
    pub fn from_float_vec(&mut self, params: &[f64]) {
        let mut idx = 0;
        for gene in &mut self.genes {
            if idx >= params.len() {
                break;
            }
            match &mut gene.gene_type {
                GeneType::Float { value, min, max } => {
                    *value = params[idx].clamp(*min, *max);
                    idx += 1;
                }
                GeneType::Integer { value, min, max } => {
                    *value = (params[idx] as i64).clamp(*min, *max);
                    idx += 1;
                }
                GeneType::Boolean { value } => {
                    *value = params[idx] > 0.5;
                    idx += 1;
                }
                GeneType::Categorical { value, options } => {
                    *value = ((params[idx] * options.len() as f64) as usize)
                        .min(options.len().saturating_sub(1));
                    idx += 1;
                }
                _ => {}
            }
        }
    }
}

impl Default for Genome {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_float_gene() {
        let mut gene = Gene::float("test", 0.5, 0.0, 1.0);
        assert_eq!(gene.as_float(), Some(0.5));

        gene.mutate(0.5);
        let val = gene.as_float().unwrap();
        assert!(val >= 0.0 && val <= 1.0);
    }

    #[test]
    fn test_binary_gene() {
        let mut gene = Gene::binary("bits", 10);
        gene.randomize();

        if let GeneType::Binary { bits } = &gene.gene_type {
            assert_eq!(bits.len(), 10);
        }
    }

    #[test]
    fn test_genome_crossover() {
        let genome1 = Genome::new_float(5, 0.0, 1.0);
        let genome2 = Genome::new_float(5, 0.0, 1.0);

        let child = genome1.crossover(&genome2);

        assert_eq!(child.genes.len(), 5);
        assert_eq!(child.parents.len(), 2);
    }

    #[test]
    fn test_genome_distance() {
        let genome1 = Genome::new_float(3, 0.0, 1.0);
        let genome2 = genome1.clone();

        assert_eq!(genome1.distance(&genome2), 0.0);
    }

    #[test]
    fn test_permutation_gene() {
        let mut gene = Gene::permutation("route", 5);
        gene.randomize();

        if let GeneType::Permutation { values } = &gene.gene_type {
            assert_eq!(values.len(), 5);
            // Check all values present
            let mut sorted = values.clone();
            sorted.sort();
            assert_eq!(sorted, vec![0, 1, 2, 3, 4]);
        }
    }
}
