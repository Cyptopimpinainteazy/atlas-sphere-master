//! Evolution engine for strategy optimization
//!
//! Implements genetic algorithms and evolutionary strategies for:
//! - Strategy parameter optimization
//! - Neural architecture search
//! - Hyperparameter tuning

mod genome;
mod mutation;
mod population;
mod selection;

pub use genome::{Gene, GeneType, Genome};
pub use mutation::{Mutation, MutationOperator};
pub use population::{Population, Species};
pub use selection::{Selection, SelectionOperator};

use crate::error::{SwarmError, SwarmResult};
use crate::types::{StrategyEvaluation, StrategyId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Evolution engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionConfig {
    /// Population size
    pub population_size: usize,
    /// Elite count (guaranteed survivors)
    pub elite_count: usize,
    /// Mutation rate (0.0 - 1.0)
    pub mutation_rate: f64,
    /// Crossover rate (0.0 - 1.0)
    pub crossover_rate: f64,
    /// Enable speciation
    pub enable_speciation: bool,
    /// Species compatibility threshold
    pub compatibility_threshold: f64,
    /// Maximum stagnation generations before extinction
    pub max_stagnation: usize,
    /// Selection operator
    pub selection: SelectionOperator,
    /// Mutation operator
    pub mutation: MutationOperator,
}

impl Default for EvolutionConfig {
    fn default() -> Self {
        Self {
            population_size: 100,
            elite_count: 5,
            mutation_rate: 0.3,
            crossover_rate: 0.7,
            enable_speciation: true,
            compatibility_threshold: 3.0,
            max_stagnation: 15,
            selection: SelectionOperator::Tournament { size: 5 },
            mutation: MutationOperator::Adaptive,
        }
    }
}

/// Evolution engine
pub struct EvolutionEngine {
    config: EvolutionConfig,
    population: Population,
    generation: u64,
    best_fitness: f64,
    best_genome: Option<Genome>,
    fitness_history: Vec<f64>,
    stagnation_counter: usize,
}

impl EvolutionEngine {
    /// Create new evolution engine
    pub fn new(config: EvolutionConfig) -> Self {
        Self {
            config: config.clone(),
            population: Population::new(config.population_size),
            generation: 0,
            best_fitness: f64::NEG_INFINITY,
            best_genome: None,
            fitness_history: Vec::new(),
            stagnation_counter: 0,
        }
    }

    /// Initialize population with random genomes
    pub fn initialize(&mut self, genome_template: &Genome) {
        self.population
            .initialize_random(genome_template, self.config.population_size);
    }

    /// Run one generation of evolution
    pub fn evolve(
        &mut self,
        fitness_scores: &HashMap<StrategyId, f64>,
    ) -> SwarmResult<EvolutionStats> {
        // Update fitness scores
        self.population.update_fitness(fitness_scores);

        // Find best
        let (gen_best_fitness, gen_best_id) = self.population.best();

        // Track improvement
        if gen_best_fitness > self.best_fitness {
            self.best_fitness = gen_best_fitness;
            self.best_genome = self.population.get(gen_best_id).cloned();
            self.stagnation_counter = 0;
        } else {
            self.stagnation_counter += 1;
        }

        self.fitness_history.push(gen_best_fitness);

        // Selection
        let selected = self.select_parents();

        // Create new population
        let mut new_pop = Vec::new();

        // Elitism: keep best individuals
        let elite = self.population.top_n(self.config.elite_count);
        for genome in elite {
            new_pop.push(genome.clone());
        }

        // Fill rest through crossover and mutation
        while new_pop.len() < self.config.population_size {
            if rand::random::<f64>() < self.config.crossover_rate && selected.len() >= 2 {
                // Crossover
                let parent1 = &selected[rand::random::<usize>() % selected.len()];
                let parent2 = &selected[rand::random::<usize>() % selected.len()];
                let mut child = parent1.crossover(parent2);

                // Possibly mutate
                if rand::random::<f64>() < self.config.mutation_rate {
                    child.mutate(&self.config.mutation);
                }
                new_pop.push(child);
            } else {
                // Mutation only
                let parent = &selected[rand::random::<usize>() % selected.len()];
                let mut child = parent.clone();
                child.mutate(&self.config.mutation);
                new_pop.push(child);
            }
        }

        // Speciation
        let species_count = if self.config.enable_speciation {
            self.population
                .speciate(self.config.compatibility_threshold)
        } else {
            1
        };

        // Handle stagnation
        let extinctions = if self.stagnation_counter >= self.config.max_stagnation {
            self.handle_stagnation()
        } else {
            0
        };

        self.population.replace_all(new_pop);
        self.generation += 1;

        Ok(EvolutionStats {
            generation: self.generation,
            population_size: self.population.len(),
            best_fitness: gen_best_fitness,
            mean_fitness: self.population.mean_fitness(),
            species_count,
            mutation_count: self.population.len() - self.config.elite_count,
            crossover_count: ((self.population.len() - self.config.elite_count) as f64
                * self.config.crossover_rate) as usize,
            extinction_count: extinctions,
            stagnation_generations: self.stagnation_counter,
        })
    }

    /// Select parents for next generation
    fn select_parents(&self) -> Vec<Genome> {
        match self.config.selection {
            SelectionOperator::Tournament { size } => self
                .population
                .tournament_select(size, self.config.population_size / 2),
            SelectionOperator::RouletteWheel => self
                .population
                .roulette_select(self.config.population_size / 2),
            SelectionOperator::Rank => self.population.rank_select(self.config.population_size / 2),
            SelectionOperator::Nsga2 => {
                // Multi-objective selection
                self.population
                    .nsga2_select(self.config.population_size / 2)
            }
        }
    }

    /// Handle stagnation by resetting or diversifying
    fn handle_stagnation(&mut self) -> usize {
        // Keep elite, replace rest with new random genomes
        let elite = self.population.top_n(self.config.elite_count);
        let extinctions = self.population.len() - elite.len();

        // Clone template before consuming elite
        let template = elite.first().cloned();
        let mut new_pop: Vec<Genome> = elite.into_iter().cloned().collect();

        if let Some(template_genome) = template {
            // Generate random individuals based on template
            while new_pop.len() < self.config.population_size {
                let mut new_genome = template_genome.clone();
                new_genome.randomize();
                new_pop.push(new_genome);
            }
        }

        self.population.replace_all(new_pop);
        self.stagnation_counter = 0;
        extinctions
    }

    /// Get best genome found
    pub fn best_genome(&self) -> Option<&Genome> {
        self.best_genome.as_ref()
    }

    /// Get best fitness
    pub fn best_fitness(&self) -> f64 {
        self.best_fitness
    }

    /// Get current generation
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Get population
    pub fn population(&self) -> &Population {
        &self.population
    }

    /// Get fitness history
    pub fn fitness_history(&self) -> &[f64] {
        &self.fitness_history
    }
}

/// Statistics from one evolution generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionStats {
    pub generation: u64,
    pub population_size: usize,
    pub best_fitness: f64,
    pub mean_fitness: f64,
    pub species_count: usize,
    pub mutation_count: usize,
    pub crossover_count: usize,
    pub extinction_count: usize,
    pub stagnation_generations: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evolution_engine() {
        let config = EvolutionConfig {
            population_size: 20,
            elite_count: 2,
            ..Default::default()
        };

        let mut engine = EvolutionEngine::new(config);

        // Create template genome
        let template = Genome::new_float(5, -1.0, 1.0);
        engine.initialize(&template);

        assert_eq!(engine.population().len(), 20);
        assert_eq!(engine.generation(), 0);
    }
}
