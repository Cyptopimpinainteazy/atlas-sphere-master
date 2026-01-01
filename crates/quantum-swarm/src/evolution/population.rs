//! Population management for evolutionary algorithms
//!
//! Handles:
//! - Population storage and indexing
//! - Speciation (NEAT-style)
//! - Fitness statistics
//! - Generation management

use super::genome::{Gene, GeneType, Genome};
use crate::types::StrategyId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Calculate distance between two sets of genes
/// Used to avoid borrow issues in speciation
fn calculate_genome_distance(genes1: &[Gene], genes2: &[Gene]) -> f64 {
    let mut total_distance = 0.0;
    let mut count = 0;

    for (g1, g2) in genes1.iter().zip(genes2.iter()) {
        let gene_dist = match (&g1.gene_type, &g2.gene_type) {
            (
                GeneType::Float {
                    value: v1,
                    min,
                    max,
                },
                GeneType::Float { value: v2, .. },
            ) => {
                let range = max - min;
                if range > 0.0 {
                    (v1 - v2).abs() / range
                } else {
                    0.0
                }
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
                if range > 0.0 {
                    (*v1 - *v2).abs() as f64 / range
                } else {
                    0.0
                }
            }
            (GeneType::Boolean { value: v1 }, GeneType::Boolean { value: v2 }) => {
                if v1 == v2 {
                    0.0
                } else {
                    1.0
                }
            }
            (GeneType::Categorical { value: v1, .. }, GeneType::Categorical { value: v2, .. }) => {
                if v1 == v2 {
                    0.0
                } else {
                    1.0
                }
            }
            _ => 0.5, // Different gene types
        };
        total_distance += gene_dist;
        count += 1;
    }

    // Handle different lengths
    let len_diff = (genes1.len() as i32 - genes2.len() as i32).unsigned_abs() as f64;
    total_distance += len_diff * 0.5;

    if count > 0 {
        total_distance / count as f64
    } else {
        0.0
    }
}

/// Species in the population
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Species {
    /// Species identifier
    pub id: usize,
    /// Representative genome
    pub representative: StrategyId,
    /// Member genome IDs
    pub members: Vec<StrategyId>,
    /// Best fitness ever achieved
    pub best_fitness: f64,
    /// Generations since improvement
    pub stagnation: usize,
    /// Average fitness
    pub avg_fitness: f64,
}

impl Species {
    /// Create new species
    pub fn new(id: usize, representative: StrategyId) -> Self {
        Self {
            id,
            representative,
            members: vec![representative],
            best_fitness: f64::NEG_INFINITY,
            stagnation: 0,
            avg_fitness: 0.0,
        }
    }

    /// Update statistics
    pub fn update_stats(&mut self, genomes: &HashMap<StrategyId, Genome>) {
        if self.members.is_empty() {
            return;
        }

        let mut total_fitness = 0.0;
        let mut max_fitness = f64::NEG_INFINITY;

        for id in &self.members {
            if let Some(genome) = genomes.get(id) {
                total_fitness += genome.fitness;
                max_fitness = max_fitness.max(genome.fitness);
            }
        }

        self.avg_fitness = total_fitness / self.members.len() as f64;

        if max_fitness > self.best_fitness {
            self.best_fitness = max_fitness;
            self.stagnation = 0;
        } else {
            self.stagnation += 1;
        }
    }
}

/// Population of genomes
#[derive(Debug, Clone, Default)]
pub struct Population {
    /// All genomes indexed by ID
    genomes: HashMap<StrategyId, Genome>,
    /// Ordered list of IDs
    ids: Vec<StrategyId>,
    /// Species
    species: Vec<Species>,
}

impl Population {
    /// Create new empty population
    pub fn new(_capacity: usize) -> Self {
        Self {
            genomes: HashMap::new(),
            ids: Vec::new(),
            species: Vec::new(),
        }
    }

    /// Initialize with random genomes based on template
    pub fn initialize_random(&mut self, template: &Genome, count: usize) {
        self.genomes.clear();
        self.ids.clear();

        for _ in 0..count {
            let mut genome = template.clone();
            genome.randomize();
            self.add(genome);
        }
    }

    /// Add a genome
    pub fn add(&mut self, genome: Genome) {
        let id = genome.id;
        self.genomes.insert(id, genome);
        self.ids.push(id);
    }

    /// Get genome by ID
    pub fn get(&self, id: StrategyId) -> Option<&Genome> {
        self.genomes.get(&id)
    }

    /// Get mutable genome by ID
    pub fn get_mut(&mut self, id: StrategyId) -> Option<&mut Genome> {
        self.genomes.get_mut(&id)
    }

    /// Population size
    pub fn len(&self) -> usize {
        self.genomes.len()
    }

    /// Is population empty
    pub fn is_empty(&self) -> bool {
        self.genomes.is_empty()
    }

    /// Update fitness scores
    pub fn update_fitness(&mut self, scores: &HashMap<StrategyId, f64>) {
        for (id, score) in scores {
            if let Some(genome) = self.genomes.get_mut(id) {
                genome.fitness = *score;
            }
        }
    }

    /// Get best genome
    pub fn best(&self) -> (f64, StrategyId) {
        let mut best_fitness = f64::NEG_INFINITY;
        let mut best_id = StrategyId::nil();

        for (id, genome) in &self.genomes {
            if genome.fitness > best_fitness {
                best_fitness = genome.fitness;
                best_id = *id;
            }
        }

        (best_fitness, best_id)
    }

    /// Get top N genomes
    pub fn top_n(&self, n: usize) -> Vec<&Genome> {
        let mut sorted: Vec<_> = self.genomes.values().collect();
        sorted.sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap());
        sorted.into_iter().take(n).collect()
    }

    /// Mean fitness
    pub fn mean_fitness(&self) -> f64 {
        if self.genomes.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.genomes.values().map(|g| g.fitness).sum();
        sum / self.genomes.len() as f64
    }

    /// Tournament selection
    pub fn tournament_select(&self, tournament_size: usize, count: usize) -> Vec<Genome> {
        let mut selected = Vec::new();
        let ids: Vec<_> = self.genomes.keys().cloned().collect();

        for _ in 0..count {
            if ids.is_empty() {
                break;
            }

            // Select tournament participants
            let mut best: Option<&Genome> = None;
            for _ in 0..tournament_size {
                let idx = rand::random::<usize>() % ids.len();
                let genome = self.genomes.get(&ids[idx]).unwrap();
                if best.is_none() || genome.fitness > best.unwrap().fitness {
                    best = Some(genome);
                }
            }

            if let Some(winner) = best {
                selected.push(winner.clone());
            }
        }

        selected
    }

    /// Roulette wheel selection
    pub fn roulette_select(&self, count: usize) -> Vec<Genome> {
        let mut selected = Vec::new();

        // Calculate total fitness (shifted to be positive)
        let min_fitness = self
            .genomes
            .values()
            .map(|g| g.fitness)
            .fold(f64::INFINITY, f64::min);

        let shift = if min_fitness < 0.0 {
            -min_fitness + 1.0
        } else {
            0.0
        };
        let total: f64 = self.genomes.values().map(|g| g.fitness + shift).sum();

        for _ in 0..count {
            if total <= 0.0 {
                // Uniform selection if all fitness is zero
                let idx = rand::random::<usize>() % self.ids.len();
                if let Some(genome) = self.genomes.get(&self.ids[idx]) {
                    selected.push(genome.clone());
                }
                continue;
            }

            let threshold = rand::random::<f64>() * total;
            let mut cumulative = 0.0;

            for genome in self.genomes.values() {
                cumulative += genome.fitness + shift;
                if cumulative >= threshold {
                    selected.push(genome.clone());
                    break;
                }
            }
        }

        selected
    }

    /// Rank-based selection
    pub fn rank_select(&self, count: usize) -> Vec<Genome> {
        let mut sorted: Vec<_> = self.genomes.values().collect();
        sorted.sort_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap());

        let n = sorted.len();
        let total_rank: usize = (1..=n).sum();
        let mut selected = Vec::new();

        for _ in 0..count {
            let threshold = rand::random::<usize>() % total_rank;
            let mut cumulative = 0;

            for (rank, genome) in sorted.iter().enumerate() {
                cumulative += rank + 1;
                if cumulative >= threshold {
                    selected.push((*genome).clone());
                    break;
                }
            }
        }

        selected
    }

    /// NSGA-II selection (multi-objective)
    pub fn nsga2_select(&self, count: usize) -> Vec<Genome> {
        // For now, fall back to tournament selection
        // Full NSGA-II would require multiple objectives
        self.tournament_select(5, count)
    }

    /// Replace all genomes
    pub fn replace_all(&mut self, new_genomes: Vec<Genome>) {
        self.genomes.clear();
        self.ids.clear();
        for genome in new_genomes {
            self.add(genome);
        }
    }

    /// Speciate population
    pub fn speciate(&mut self, threshold: f64) -> usize {
        self.species.clear();
        let mut species_id = 0;

        // First pass: collect genome distances to species representatives
        // Clone IDs to avoid borrow issues
        let ids: Vec<_> = self.ids.clone();

        for id in &ids {
            if let Some(genome) = self.genomes.get(id) {
                let genome_genes = genome.genes.clone();
                let mut found_species = false;
                let mut assign_to_species: Option<(usize, usize)> = None;

                // Try to find compatible species
                for (si, species) in self.species.iter().enumerate() {
                    if let Some(rep) = self.genomes.get(&species.representative) {
                        // Calculate distance using cloned data
                        let distance = calculate_genome_distance(&genome_genes, &rep.genes);
                        if distance < threshold {
                            assign_to_species = Some((si, species.id));
                            found_species = true;
                            break;
                        }
                    }
                }

                // Now apply the mutation
                if let Some((si, sp_id)) = assign_to_species {
                    self.species[si].members.push(*id);
                    if let Some(g) = self.genomes.get_mut(id) {
                        g.species_id = Some(sp_id);
                    }
                }

                // Create new species if not found
                if !found_species {
                    let mut new_species = Species::new(species_id, *id);
                    new_species.members.clear();
                    new_species.members.push(*id);
                    self.species.push(new_species);
                    if let Some(g) = self.genomes.get_mut(id) {
                        g.species_id = Some(species_id);
                    }
                    species_id += 1;
                }
            }
        }

        // Update species stats
        let genomes_clone = self.genomes.clone();
        for species in &mut self.species {
            species.update_stats(&genomes_clone);
        }

        self.species.len()
    }

    /// Get all genomes as iterator
    pub fn iter(&self) -> impl Iterator<Item = &Genome> {
        self.genomes.values()
    }

    /// Get species
    pub fn species(&self) -> &[Species] {
        &self.species
    }
}

#[cfg(test)]
mod tests {
    use super::super::genome::Genome;
    use super::*;

    #[test]
    fn test_population() {
        let mut pop = Population::new(100);
        let template = Genome::new_float(5, 0.0, 1.0);

        pop.initialize_random(&template, 20);

        assert_eq!(pop.len(), 20);
    }

    #[test]
    fn test_tournament_selection() {
        let mut pop = Population::new(100);
        let template = Genome::new_float(5, 0.0, 1.0);
        pop.initialize_random(&template, 20);

        // Assign random fitness
        let mut scores = HashMap::new();
        for id in &pop.ids.clone() {
            scores.insert(*id, rand::random::<f64>());
        }
        pop.update_fitness(&scores);

        let selected = pop.tournament_select(5, 10);
        assert_eq!(selected.len(), 10);
    }

    #[test]
    fn test_speciation() {
        let mut pop = Population::new(100);
        let template = Genome::new_float(5, 0.0, 1.0);
        pop.initialize_random(&template, 20);

        let num_species = pop.speciate(0.5);
        assert!(num_species > 0);
    }
}
