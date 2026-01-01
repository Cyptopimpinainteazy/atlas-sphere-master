//! X3 Evolution Engine - Genetic Algorithm for Strategy Optimization
//!
//! This crate provides:
//! - Safe bytecode mutation operators
//! - Genetic crossover between strategies
//! - PnL-based fitness scoring
//! - Population management with elitism
//! - Tournament and roulette selection
//! - Deterministic evolution for reproducibility
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
//! │  Strategy   │───▶│  Mutation   │───▶│ Simulation  │
//! │  Population │    │  Operators  │    │   Engine    │
//! └─────────────┘    └─────────────┘    └─────────────┘
//!                                              │
//! ┌─────────────┐    ┌─────────────┐           ▼
//! │   Next Gen  │◀───│  Selection  │◀───┌─────────────┐
//! │  Population │    │  (Elite)    │    │   Fitness   │
//! └─────────────┘    └─────────────┘    │   Scoring   │
//!                                       └─────────────┘
//! ```

pub mod chromosome;
pub mod crossover;
pub mod error;
pub mod fitness;
pub mod mutation;
pub mod population;
pub mod selection;
pub mod simulator;

pub use chromosome::{Chromosome, ChromosomeMetadata, Gene, GeneConstraints, GeneType};
pub use crossover::{
    AdaptiveCrossover, ArithmeticCrossover, CrossoverOperator, SinglePointCrossover,
    TwoPointCrossover, UniformCrossover,
};
pub use error::{EvolutionError, Result};
pub use fitness::{FitnessEvaluator, FitnessScore, MockFitness, PnLFitness};
pub use mutation::{
    CompositeMutation, GaussianMutation, LogicMutation, MutationOperator, ParameterMutation,
    SwapMutation,
};
pub use population::{Individual, Population, PopulationSeeder, PopulationStats};
pub use selection::{
    EliteSelection, RankSelection, RouletteSelection, SelectionOperator,
    StochasticUniversalSampling, TournamentSelection, TruncationSelection,
};
pub use simulator::{
    generate_synthetic_data, MarketTick, PortfolioState, SimulationConfig, SimulationResult,
    Simulator, SimulatorFitness, Trade, TradeAction,
};

use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use tracing::{debug, info, warn};

/// Evolution engine configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EvolutionConfig {
    /// Population size
    pub population_size: usize,
    /// Number of generations to evolve
    pub generations: usize,
    /// Mutation rate (0.0 - 1.0)
    pub mutation_rate: f64,
    /// Crossover rate (0.0 - 1.0)
    pub crossover_rate: f64,
    /// Elite percentage to preserve (0.0 - 1.0)
    pub elite_ratio: f64,
    /// Tournament size for selection
    pub tournament_size: usize,
    /// Random seed for deterministic evolution
    pub seed: Option<u64>,
    /// Number of simulation runs per strategy
    pub simulations_per_strategy: usize,
    /// Maximum strategy bytecode size
    pub max_bytecode_size: usize,
    /// Enable parallel evaluation
    pub parallel: bool,
}

impl Default for EvolutionConfig {
    fn default() -> Self {
        Self {
            population_size: 100,
            generations: 50,
            mutation_rate: 0.1,
            crossover_rate: 0.7,
            elite_ratio: 0.1,
            tournament_size: 5,
            seed: None,
            simulations_per_strategy: 1000,
            max_bytecode_size: 16384, // 16KB
            parallel: true,
        }
    }
}

/// Evolution engine for X3 strategies
pub struct EvolutionEngine {
    config: EvolutionConfig,
    rng: ChaCha20Rng,
    population: Population,
    generation: usize,
    best_fitness: f64,
    fitness_history: Vec<f64>,
}

impl EvolutionEngine {
    /// Create a new evolution engine
    pub fn new(config: EvolutionConfig) -> Self {
        let seed = config.seed.unwrap_or_else(|| rand::random());
        let rng = ChaCha20Rng::seed_from_u64(seed);

        info!(
            population_size = config.population_size,
            generations = config.generations,
            mutation_rate = config.mutation_rate,
            seed = seed,
            "Initializing evolution engine"
        );

        Self {
            population: Population::new(config.population_size),
            config,
            rng,
            generation: 0,
            best_fitness: f64::NEG_INFINITY,
            fitness_history: Vec::new(),
        }
    }

    /// Initialize population with seed strategies
    pub fn seed_population(&mut self, seed_bytecode: Vec<Vec<u8>>) -> Result<()> {
        info!(seed_count = seed_bytecode.len(), "Seeding population");

        for bytecode in seed_bytecode {
            if bytecode.len() > self.config.max_bytecode_size {
                return Err(EvolutionError::BytecodeTooLarge {
                    size: bytecode.len(),
                    max: self.config.max_bytecode_size,
                });
            }

            let chromosome = Chromosome::from_bytecode(bytecode)?;
            self.population.add(Individual::new(chromosome));
        }

        // Fill remaining slots with mutations of seeds
        while self.population.len() < self.config.population_size {
            let parent = self.population.random_individual(&mut self.rng)?;
            let mut child = parent.chromosome.clone();

            // Apply random mutations
            let mutation = ParameterMutation::new(self.config.mutation_rate);
            mutation.mutate(&mut child, &mut self.rng)?;

            self.population.add(Individual::new(child));
        }

        Ok(())
    }

    /// Run a single generation of evolution
    pub fn evolve_generation<E: FitnessEvaluator>(
        &mut self,
        evaluator: &E,
    ) -> Result<GenerationStats> {
        self.generation += 1;
        debug!(generation = self.generation, "Starting generation");

        // 1. Evaluate fitness for all individuals
        self.evaluate_fitness(evaluator)?;

        // 2. Record stats
        let stats = self.compute_stats();
        self.fitness_history.push(stats.best_fitness);

        if stats.best_fitness > self.best_fitness {
            self.best_fitness = stats.best_fitness;
            info!(
                generation = self.generation,
                best_fitness = self.best_fitness,
                "New best fitness found!"
            );
        }

        // 3. Selection
        let elite_count = (self.config.population_size as f64 * self.config.elite_ratio) as usize;
        let elite = EliteSelection::new(elite_count);
        let tournament = TournamentSelection::new(self.config.tournament_size);

        let mut next_gen = Population::new(self.config.population_size);

        // Preserve elite
        for individual in elite.select(&self.population, elite_count, &mut self.rng)? {
            next_gen.add(individual);
        }

        // 4. Crossover and mutation to fill rest
        while next_gen.len() < self.config.population_size {
            // Select parents
            let parents = tournament.select(&self.population, 2, &mut self.rng)?;
            let parent1 = &parents[0];
            let parent2 = &parents[1];

            // Crossover
            let mut child = if rand::random::<f64>() < self.config.crossover_rate {
                let crossover = UniformCrossover::new(0.5);
                crossover.crossover(&parent1.chromosome, &parent2.chromosome, &mut self.rng)?
            } else {
                parent1.chromosome.clone()
            };

            // Mutation
            if rand::random::<f64>() < self.config.mutation_rate {
                let mutation = ParameterMutation::new(self.config.mutation_rate);
                mutation.mutate(&mut child, &mut self.rng)?;
            }

            // Validate bytecode size
            if child.to_bytecode().len() <= self.config.max_bytecode_size {
                next_gen.add(Individual::new(child));
            }
        }

        self.population = next_gen;

        Ok(stats)
    }

    /// Run full evolution
    pub fn evolve<E: FitnessEvaluator>(&mut self, evaluator: &E) -> Result<EvolutionResult> {
        info!(
            generations = self.config.generations,
            population_size = self.config.population_size,
            "Starting evolution"
        );

        let mut all_stats = Vec::new();

        for _ in 0..self.config.generations {
            let stats = self.evolve_generation(evaluator)?;
            all_stats.push(stats);

            // Early termination check
            if self.should_terminate(&all_stats) {
                info!(
                    generation = self.generation,
                    "Early termination - convergence detected"
                );
                break;
            }
        }

        // Get best individual
        let best = self.population.best_individual()?.clone();

        Ok(EvolutionResult {
            best_individual: best,
            best_fitness: self.best_fitness,
            generations_run: self.generation,
            fitness_history: self.fitness_history.clone(),
            generation_stats: all_stats,
        })
    }

    fn evaluate_fitness<E: FitnessEvaluator>(&mut self, evaluator: &E) -> Result<()> {
        if self.config.parallel {
            #[cfg(feature = "parallel")]
            {
                use rayon::prelude::*;
                self.population
                    .individuals_mut()
                    .par_iter_mut()
                    .for_each(|ind| {
                        if let Ok(fitness) = evaluator.evaluate(&ind.chromosome) {
                            ind.fitness = Some(fitness);
                        }
                    });
            }
            #[cfg(not(feature = "parallel"))]
            {
                for ind in self.population.individuals_mut() {
                    ind.fitness = Some(evaluator.evaluate(&ind.chromosome)?);
                }
            }
        } else {
            for ind in self.population.individuals_mut() {
                ind.fitness = Some(evaluator.evaluate(&ind.chromosome)?);
            }
        }
        Ok(())
    }

    fn compute_stats(&self) -> GenerationStats {
        let fitnesses: Vec<f64> = self
            .population
            .individuals()
            .iter()
            .filter_map(|i| i.fitness.as_ref().map(|f| f.total_score()))
            .collect();

        let best = fitnesses.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let worst = fitnesses.iter().cloned().fold(f64::INFINITY, f64::min);
        let avg = fitnesses.iter().sum::<f64>() / fitnesses.len() as f64;
        let variance =
            fitnesses.iter().map(|f| (f - avg).powi(2)).sum::<f64>() / fitnesses.len() as f64;

        GenerationStats {
            generation: self.generation,
            best_fitness: best,
            worst_fitness: worst,
            average_fitness: avg,
            fitness_variance: variance,
            population_size: self.population.len(),
        }
    }

    fn should_terminate(&self, stats: &[GenerationStats]) -> bool {
        if stats.len() < 10 {
            return false;
        }

        // Check if fitness has plateaued
        let recent: Vec<f64> = stats
            .iter()
            .rev()
            .take(10)
            .map(|s| s.best_fitness)
            .collect();
        let variance = {
            let avg = recent.iter().sum::<f64>() / recent.len() as f64;
            recent.iter().map(|f| (f - avg).powi(2)).sum::<f64>() / recent.len() as f64
        };

        variance < 0.0001 // Converged
    }

    /// Get current best individual
    pub fn best(&self) -> Result<&Individual> {
        self.population.best_individual()
    }

    /// Get current generation number
    pub fn generation(&self) -> usize {
        self.generation
    }

    /// Get fitness history
    pub fn fitness_history(&self) -> &[f64] {
        &self.fitness_history
    }
}

/// Statistics for a single generation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GenerationStats {
    pub generation: usize,
    pub best_fitness: f64,
    pub worst_fitness: f64,
    pub average_fitness: f64,
    pub fitness_variance: f64,
    pub population_size: usize,
}

/// Result of a complete evolution run
#[derive(Debug, Clone)]
pub struct EvolutionResult {
    pub best_individual: Individual,
    pub best_fitness: f64,
    pub generations_run: usize,
    pub fitness_history: Vec<f64>,
    pub generation_stats: Vec<GenerationStats>,
}

impl EvolutionResult {
    /// Get the best strategy bytecode
    pub fn best_bytecode(&self) -> Vec<u8> {
        self.best_individual.chromosome.to_bytecode()
    }

    /// Get deterministic hash of the result
    pub fn result_hash(&self) -> [u8; 32] {
        let bytecode = self.best_bytecode();
        blake3::hash(&bytecode).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evolution_config_default() {
        let config = EvolutionConfig::default();
        assert_eq!(config.population_size, 100);
        assert_eq!(config.generations, 50);
        assert!(config.mutation_rate > 0.0);
    }

    #[test]
    fn test_evolution_engine_creation() {
        let config = EvolutionConfig {
            seed: Some(42),
            ..Default::default()
        };
        let engine = EvolutionEngine::new(config);
        assert_eq!(engine.generation(), 0);
    }
}
