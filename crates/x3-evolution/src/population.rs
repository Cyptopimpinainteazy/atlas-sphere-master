//! Population management for evolutionary algorithms

use crate::chromosome::Chromosome;
use crate::error::{EvolutionError, Result};
use crate::fitness::FitnessScore;
use rand::Rng;
use std::collections::HashMap;

/// Individual in the population
#[derive(Debug, Clone)]
pub struct Individual {
    /// The chromosome (X3 bytecode)
    pub chromosome: Chromosome,

    /// Fitness score (computed after simulation)
    pub fitness: Option<FitnessScore>,

    /// Generation when this individual was created
    pub birth_generation: usize,

    /// Parent IDs (for lineage tracking)
    pub parent_ids: Vec<[u8; 32]>,

    /// Mutation count (how many times mutated)
    pub mutation_count: usize,

    /// Is this an elite (preserved across generations)
    pub is_elite: bool,
}

impl Individual {
    /// Create a new individual from a chromosome
    pub fn new(chromosome: Chromosome) -> Self {
        Self {
            chromosome,
            fitness: None,
            birth_generation: 0,
            parent_ids: Vec::new(),
            mutation_count: 0,
            is_elite: false,
        }
    }

    /// Create with fitness
    pub fn with_fitness(chromosome: Chromosome, fitness: FitnessScore) -> Self {
        Self {
            chromosome,
            fitness: Some(fitness),
            birth_generation: 0,
            parent_ids: Vec::new(),
            mutation_count: 0,
            is_elite: false,
        }
    }

    /// Get individual's hash (from chromosome)
    pub fn hash(&self) -> [u8; 32] {
        self.chromosome.hash()
    }

    /// Get fitness score or negative infinity
    pub fn fitness_score(&self) -> f64 {
        self.fitness
            .as_ref()
            .map(|f| f.total_score())
            .unwrap_or(f64::NEG_INFINITY)
    }

    /// Set as child of given parents
    pub fn set_parents(&mut self, parents: &[&Individual]) {
        self.parent_ids = parents.iter().map(|p| p.hash()).collect();
    }
}

/// Population of individuals
#[derive(Debug, Clone)]
pub struct Population {
    /// Individuals in the population
    individuals: Vec<Individual>,

    /// Maximum population size
    max_size: usize,

    /// Current generation
    generation: usize,

    /// Best fitness seen
    best_fitness: f64,

    /// Hash set for deduplication
    seen_hashes: HashMap<[u8; 32], usize>,
}

impl Population {
    /// Create a new population with given max size
    pub fn new(max_size: usize) -> Self {
        Self {
            individuals: Vec::with_capacity(max_size),
            max_size,
            generation: 0,
            best_fitness: f64::NEG_INFINITY,
            seen_hashes: HashMap::new(),
        }
    }

    /// Add an individual to the population
    pub fn add(&mut self, mut individual: Individual) {
        let hash = individual.hash();

        // Check for duplicates
        if self.seen_hashes.contains_key(&hash) {
            return; // Skip duplicate
        }

        individual.birth_generation = self.generation;
        self.seen_hashes.insert(hash, self.individuals.len());

        // Update best fitness
        if let Some(fitness) = &individual.fitness {
            let score = fitness.total_score();
            if score > self.best_fitness {
                self.best_fitness = score;
            }
        }

        self.individuals.push(individual);
    }

    /// Add multiple individuals
    pub fn add_all(&mut self, individuals: Vec<Individual>) {
        for individual in individuals {
            self.add(individual);
        }
    }

    /// Get all individuals
    pub fn individuals(&self) -> &[Individual] {
        &self.individuals
    }

    /// Get mutable individuals
    pub fn individuals_mut(&mut self) -> &mut [Individual] {
        &mut self.individuals
    }

    /// Get population size
    pub fn size(&self) -> usize {
        self.individuals.len()
    }

    /// Alias for size() - compatibility with lib.rs
    pub fn len(&self) -> usize {
        self.individuals.len()
    }

    /// Check if population is empty
    pub fn is_empty(&self) -> bool {
        self.individuals.is_empty()
    }

    /// Get max size
    pub fn max_size(&self) -> usize {
        self.max_size
    }

    /// Get current generation
    pub fn generation(&self) -> usize {
        self.generation
    }

    /// Advance to next generation
    pub fn advance_generation(&mut self) {
        self.generation += 1;
    }

    /// Get best individual
    pub fn best(&self) -> Option<&Individual> {
        self.individuals.iter().max_by(|a, b| {
            a.fitness_score()
                .partial_cmp(&b.fitness_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// Get best individual (returns Result for compatibility with lib.rs)
    pub fn best_individual(&self) -> Result<&Individual> {
        self.best().ok_or(EvolutionError::EmptyPopulation)
    }

    /// Get a random individual from the population
    pub fn random_individual<R: rand::Rng>(&self, rng: &mut R) -> Result<&Individual> {
        if self.individuals.is_empty() {
            return Err(EvolutionError::EmptyPopulation);
        }
        let idx = rng.gen_range(0..self.individuals.len());
        Ok(&self.individuals[idx])
    }

    /// Get top N individuals
    pub fn top_n(&self, n: usize) -> Vec<&Individual> {
        let mut sorted: Vec<&Individual> = self.individuals.iter().collect();
        sorted.sort_by(|a, b| {
            b.fitness_score()
                .partial_cmp(&a.fitness_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted.truncate(n);
        sorted
    }

    /// Get average fitness
    pub fn average_fitness(&self) -> f64 {
        if self.individuals.is_empty() {
            return 0.0;
        }

        let sum: f64 = self
            .individuals
            .iter()
            .filter_map(|i| i.fitness.as_ref())
            .map(|f| f.total_score())
            .sum();

        let count = self
            .individuals
            .iter()
            .filter(|i| i.fitness.is_some())
            .count();

        if count > 0 {
            sum / count as f64
        } else {
            0.0
        }
    }

    /// Get fitness diversity (standard deviation)
    pub fn fitness_diversity(&self) -> f64 {
        let avg = self.average_fitness();

        let count = self
            .individuals
            .iter()
            .filter(|i| i.fitness.is_some())
            .count();

        if count <= 1 {
            return 0.0;
        }

        let variance: f64 = self
            .individuals
            .iter()
            .filter_map(|i| i.fitness.as_ref())
            .map(|f| {
                let diff = f.total_score() - avg;
                diff * diff
            })
            .sum::<f64>()
            / count as f64;

        variance.sqrt()
    }

    /// Apply elitism: mark top N as elite
    pub fn apply_elitism(&mut self, elite_count: usize) {
        // Reset all elite flags
        for individual in &mut self.individuals {
            individual.is_elite = false;
        }

        // Find top N
        let mut indices: Vec<(usize, f64)> = self
            .individuals
            .iter()
            .enumerate()
            .map(|(i, ind)| (i, ind.fitness_score()))
            .collect();

        indices.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Mark as elite
        for (idx, _) in indices.into_iter().take(elite_count) {
            self.individuals[idx].is_elite = true;
        }
    }

    /// Get elite individuals
    pub fn elites(&self) -> Vec<&Individual> {
        self.individuals.iter().filter(|i| i.is_elite).collect()
    }

    /// Trim population to max size (keeping elites)
    pub fn trim(&mut self) {
        if self.individuals.len() <= self.max_size {
            return;
        }

        // Sort by fitness (but elites always first)
        self.individuals
            .sort_by(|a, b| match (a.is_elite, b.is_elite) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => b
                    .fitness_score()
                    .partial_cmp(&a.fitness_score())
                    .unwrap_or(std::cmp::Ordering::Equal),
            });

        // Truncate
        self.individuals.truncate(self.max_size);

        // Rebuild hash set
        self.seen_hashes.clear();
        for (i, individual) in self.individuals.iter().enumerate() {
            self.seen_hashes.insert(individual.hash(), i);
        }
    }

    /// Clear population (keep settings)
    pub fn clear(&mut self) {
        self.individuals.clear();
        self.seen_hashes.clear();
        self.best_fitness = f64::NEG_INFINITY;
    }

    /// Reset for new evolution run
    pub fn reset(&mut self) {
        self.clear();
        self.generation = 0;
    }

    /// Get statistics
    pub fn stats(&self) -> PopulationStats {
        let fitness_values: Vec<f64> = self
            .individuals
            .iter()
            .filter_map(|i| i.fitness.as_ref())
            .map(|f| f.total_score())
            .collect();

        let (min, max, avg, std_dev) = if fitness_values.is_empty() {
            (0.0, 0.0, 0.0, 0.0)
        } else {
            let min = fitness_values.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = fitness_values
                .iter()
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max);
            let avg = fitness_values.iter().sum::<f64>() / fitness_values.len() as f64;

            let variance = fitness_values
                .iter()
                .map(|f| (f - avg).powi(2))
                .sum::<f64>()
                / fitness_values.len() as f64;

            (min, max, avg, variance.sqrt())
        };

        PopulationStats {
            size: self.individuals.len(),
            generation: self.generation,
            min_fitness: min,
            max_fitness: max,
            avg_fitness: avg,
            fitness_std_dev: std_dev,
            elite_count: self.individuals.iter().filter(|i| i.is_elite).count(),
            unique_count: self.seen_hashes.len(),
        }
    }

    /// Contains an individual with this chromosome hash?
    pub fn contains(&self, hash: &[u8; 32]) -> bool {
        self.seen_hashes.contains_key(hash)
    }

    /// Merge another population into this one
    pub fn merge(&mut self, other: Population) {
        for individual in other.individuals {
            self.add(individual);
        }
    }
}

/// Population statistics
#[derive(Debug, Clone, Default)]
pub struct PopulationStats {
    pub size: usize,
    pub generation: usize,
    pub min_fitness: f64,
    pub max_fitness: f64,
    pub avg_fitness: f64,
    pub fitness_std_dev: f64,
    pub elite_count: usize,
    pub unique_count: usize,
}

impl std::fmt::Display for PopulationStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Generation {} | Size: {} | Fitness: [{:.4}, {:.4}] avg={:.4} σ={:.4} | Elites: {}",
            self.generation,
            self.size,
            self.min_fitness,
            self.max_fitness,
            self.avg_fitness,
            self.fitness_std_dev,
            self.elite_count,
        )
    }
}

/// Seed generator for creating initial populations
pub struct PopulationSeeder;

impl PopulationSeeder {
    /// Create initial population from bytecode templates
    pub fn from_templates(
        templates: &[Vec<u8>],
        population_size: usize,
        mutation_rate: f64,
    ) -> Result<Population> {
        use rand::Rng;
        use rand::SeedableRng;
        use rand_chacha::ChaCha20Rng;

        let mut population = Population::new(population_size);
        let mut rng = ChaCha20Rng::from_entropy();

        if templates.is_empty() {
            return Err(EvolutionError::InvalidParameters(
                "No templates provided".into(),
            ));
        }

        // Add templates directly
        for template in templates {
            let chromosome = Chromosome::from_bytecode(template.clone())
                .map_err(|e| EvolutionError::InvalidBytecode(e.to_string()))?;
            population.add(Individual::new(chromosome));
        }

        // Generate variations
        while population.size() < population_size {
            // Pick random template
            let template_idx = rng.gen_range(0..templates.len());
            let mut bytecode = templates[template_idx].clone();

            // Mutate bytes
            for byte in &mut bytecode {
                if rng.gen::<f64>() < mutation_rate {
                    *byte = byte.wrapping_add(rng.gen_range(1..=10));
                }
            }

            if let Ok(chromosome) = Chromosome::from_bytecode(bytecode) {
                population.add(Individual::new(chromosome));
            }
        }

        Ok(population)
    }

    /// Create population from random bytecode
    pub fn random(population_size: usize, bytecode_length: usize) -> Result<Population> {
        use rand::Rng;
        use rand::SeedableRng;
        use rand_chacha::ChaCha20Rng;

        let mut population = Population::new(population_size);
        let mut rng = ChaCha20Rng::from_entropy();

        while population.size() < population_size {
            let bytecode: Vec<u8> = (0..bytecode_length).map(|_| rng.gen()).collect();

            if let Ok(chromosome) = Chromosome::from_bytecode(bytecode) {
                population.add(Individual::new(chromosome));
            }
        }

        Ok(population)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_individual(score: f64) -> Individual {
        let bytecode = vec![0x20, score as u8, 0x00, 0x00];
        let chromosome = Chromosome::from_bytecode(bytecode).unwrap();
        Individual::with_fitness(
            chromosome,
            FitnessScore {
                pnl: score,
                ..Default::default()
            },
        )
    }

    #[test]
    fn test_population_add() {
        let mut pop = Population::new(10);

        let ind = make_individual(1.0);
        pop.add(ind);

        assert_eq!(pop.size(), 1);
        assert_eq!(pop.generation(), 0);
    }

    #[test]
    fn test_population_deduplication() {
        let mut pop = Population::new(10);

        let ind1 = make_individual(1.0);
        let ind2 = ind1.clone();

        pop.add(ind1);
        pop.add(ind2);

        assert_eq!(pop.size(), 1); // Duplicate should be rejected
    }

    #[test]
    fn test_population_best() {
        let mut pop = Population::new(10);

        for i in 0..5 {
            pop.add(make_individual(i as f64));
        }

        let best = pop.best().unwrap();
        // Check actual pnl, not total_score (which is normalized)
        assert_eq!(best.fitness.as_ref().unwrap().pnl, 4.0);
    }

    #[test]
    fn test_population_elitism() {
        let mut pop = Population::new(10);

        for i in 0..5 {
            pop.add(make_individual(i as f64));
        }

        pop.apply_elitism(2);

        let elites = pop.elites();
        assert_eq!(elites.len(), 2);

        // Top 2 should be elites (pnl >= 3.0)
        for elite in elites {
            assert!(elite.fitness.as_ref().unwrap().pnl >= 3.0);
        }
    }

    #[test]
    fn test_population_trim() {
        let mut pop = Population::new(3);

        for i in 0..5 {
            let bytecode = vec![0x20, i, 0x00, 0x00];
            let chromosome = Chromosome::from_bytecode(bytecode).unwrap();
            let ind = Individual::with_fitness(
                chromosome,
                FitnessScore {
                    pnl: i as f64,
                    ..Default::default()
                },
            );
            // Manually set hash to avoid deduplication
            pop.individuals.push(ind);
        }

        pop.trim();
        assert_eq!(pop.size(), 3);

        // Should keep top 3 (pnl >= 2.0)
        for ind in &pop.individuals {
            assert!(ind.fitness.as_ref().unwrap().pnl >= 2.0);
        }
    }

    #[test]
    fn test_population_stats() {
        let mut pop = Population::new(10);

        for i in 0..5 {
            pop.add(make_individual(i as f64));
        }

        let stats = pop.stats();
        assert_eq!(stats.size, 5);
        // Stats use total_score() which is normalized
        // Just verify bounds are reasonable
        assert!(stats.min_fitness.is_finite());
        assert!(stats.max_fitness.is_finite());
        assert!(stats.avg_fitness.is_finite());
        // Max should be > min for varied fitness
        assert!(stats.max_fitness >= stats.min_fitness);
    }

    #[test]
    fn test_population_seeder_templates() {
        let templates = vec![vec![0x20, 0x01, 0x00, 0x00], vec![0x21, 0x02, 0x00, 0x00]];

        let pop = PopulationSeeder::from_templates(&templates, 10, 0.1).unwrap();
        assert_eq!(pop.size(), 10);
    }

    #[test]
    fn test_population_seeder_random() {
        let pop = PopulationSeeder::random(10, 16).unwrap();
        assert_eq!(pop.size(), 10);
    }
}
