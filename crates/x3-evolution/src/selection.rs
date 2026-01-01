//! Selection operators for evolutionary algorithms

use crate::chromosome::Chromosome;
use crate::error::{EvolutionError, Result};
use crate::population::Individual;
use crate::Population;
use rand::Rng;
use rand_chacha::ChaCha20Rng;

/// Trait for selection operators
pub trait SelectionOperator: Send + Sync {
    /// Select individuals from population
    fn select(
        &self,
        population: &Population,
        count: usize,
        rng: &mut ChaCha20Rng,
    ) -> Result<Vec<Individual>>;

    /// Get selection operator name
    fn name(&self) -> &'static str;
}

/// Tournament selection - select best from random subgroup
pub struct TournamentSelection {
    /// Tournament size
    tournament_size: usize,
}

impl TournamentSelection {
    pub fn new(tournament_size: usize) -> Self {
        Self {
            tournament_size: tournament_size.max(2),
        }
    }
}

impl SelectionOperator for TournamentSelection {
    fn select(
        &self,
        population: &Population,
        count: usize,
        rng: &mut ChaCha20Rng,
    ) -> Result<Vec<Individual>> {
        if population.is_empty() {
            return Err(EvolutionError::EmptyPopulation);
        }

        let mut selected = Vec::with_capacity(count);
        let individuals = population.individuals();

        for _ in 0..count {
            // Run tournament
            let mut best: Option<&Individual> = None;
            let mut best_fitness = f64::NEG_INFINITY;

            for _ in 0..self.tournament_size {
                let idx = rng.gen_range(0..individuals.len());
                let candidate = &individuals[idx];

                let fitness = candidate
                    .fitness
                    .as_ref()
                    .map(|f| f.total_score())
                    .unwrap_or(f64::NEG_INFINITY);

                if fitness > best_fitness {
                    best_fitness = fitness;
                    best = Some(candidate);
                }
            }

            if let Some(winner) = best {
                selected.push(winner.clone());
            }
        }

        Ok(selected)
    }

    fn name(&self) -> &'static str {
        "TournamentSelection"
    }
}

/// Roulette wheel selection - probability proportional to fitness
pub struct RouletteSelection;

impl RouletteSelection {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RouletteSelection {
    fn default() -> Self {
        Self::new()
    }
}

impl SelectionOperator for RouletteSelection {
    fn select(
        &self,
        population: &Population,
        count: usize,
        rng: &mut ChaCha20Rng,
    ) -> Result<Vec<Individual>> {
        if population.is_empty() {
            return Err(EvolutionError::EmptyPopulation);
        }

        let individuals = population.individuals();

        // Calculate fitness values (shift to positive)
        let fitness_values: Vec<f64> = individuals
            .iter()
            .map(|i| i.fitness.as_ref().map(|f| f.total_score()).unwrap_or(0.0))
            .collect();

        let min_fitness = fitness_values.iter().cloned().fold(f64::INFINITY, f64::min);
        let shifted: Vec<f64> = fitness_values
            .iter()
            .map(|f| f - min_fitness + 1.0) // +1 to ensure positive
            .collect();

        let total: f64 = shifted.iter().sum();

        if total <= 0.0 {
            // Fall back to random selection
            let mut selected = Vec::with_capacity(count);
            for _ in 0..count {
                let idx = rng.gen_range(0..individuals.len());
                selected.push(individuals[idx].clone());
            }
            return Ok(selected);
        }

        // Cumulative probabilities
        let mut cumulative = Vec::with_capacity(shifted.len());
        let mut sum = 0.0;
        for f in &shifted {
            sum += f / total;
            cumulative.push(sum);
        }

        // Select individuals
        let mut selected = Vec::with_capacity(count);
        for _ in 0..count {
            let r = rng.gen::<f64>();
            let idx = cumulative
                .iter()
                .position(|&c| c >= r)
                .unwrap_or(individuals.len() - 1);
            selected.push(individuals[idx].clone());
        }

        Ok(selected)
    }

    fn name(&self) -> &'static str {
        "RouletteSelection"
    }
}

/// Elite selection - select top N individuals
pub struct EliteSelection {
    /// Number of elite individuals
    elite_count: usize,
}

impl EliteSelection {
    pub fn new(elite_count: usize) -> Self {
        Self { elite_count }
    }
}

impl SelectionOperator for EliteSelection {
    fn select(
        &self,
        population: &Population,
        count: usize,
        _rng: &mut ChaCha20Rng,
    ) -> Result<Vec<Individual>> {
        if population.is_empty() {
            return Err(EvolutionError::EmptyPopulation);
        }

        let mut individuals: Vec<Individual> = population.individuals().to_vec();

        // Sort by fitness (descending)
        individuals.sort_by(|a, b| {
            let fa = a
                .fitness
                .as_ref()
                .map(|f| f.total_score())
                .unwrap_or(f64::NEG_INFINITY);
            let fb = b
                .fitness
                .as_ref()
                .map(|f| f.total_score())
                .unwrap_or(f64::NEG_INFINITY);
            fb.partial_cmp(&fa).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Take top N
        let take_count = count.min(self.elite_count).min(individuals.len());
        Ok(individuals.into_iter().take(take_count).collect())
    }

    fn name(&self) -> &'static str {
        "EliteSelection"
    }
}

/// Rank selection - probability based on rank rather than fitness
pub struct RankSelection;

impl RankSelection {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RankSelection {
    fn default() -> Self {
        Self::new()
    }
}

impl SelectionOperator for RankSelection {
    fn select(
        &self,
        population: &Population,
        count: usize,
        rng: &mut ChaCha20Rng,
    ) -> Result<Vec<Individual>> {
        if population.is_empty() {
            return Err(EvolutionError::EmptyPopulation);
        }

        let mut individuals: Vec<(usize, &Individual)> =
            population.individuals().iter().enumerate().collect();

        // Sort by fitness
        individuals.sort_by(|(_, a), (_, b)| {
            let fa = a
                .fitness
                .as_ref()
                .map(|f| f.total_score())
                .unwrap_or(f64::NEG_INFINITY);
            let fb = b
                .fitness
                .as_ref()
                .map(|f| f.total_score())
                .unwrap_or(f64::NEG_INFINITY);
            fb.partial_cmp(&fa).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Assign ranks (1 to N, where 1 is worst)
        let n = individuals.len();
        let total_rank: usize = (n * (n + 1)) / 2;

        // Cumulative probabilities based on rank
        let mut cumulative = Vec::with_capacity(n);
        let mut sum = 0.0;
        for (rank, _) in individuals.iter().enumerate() {
            let rank_value = n - rank; // Higher rank for better fitness
            sum += rank_value as f64 / total_rank as f64;
            cumulative.push(sum);
        }

        // Select
        let mut selected = Vec::with_capacity(count);
        for _ in 0..count {
            let r = rng.gen::<f64>();
            let idx = cumulative.iter().position(|&c| c >= r).unwrap_or(n - 1);
            selected.push(individuals[idx].1.clone());
        }

        Ok(selected)
    }

    fn name(&self) -> &'static str {
        "RankSelection"
    }
}

/// Stochastic Universal Sampling (SUS)
pub struct StochasticUniversalSampling;

impl StochasticUniversalSampling {
    pub fn new() -> Self {
        Self
    }
}

impl Default for StochasticUniversalSampling {
    fn default() -> Self {
        Self::new()
    }
}

impl SelectionOperator for StochasticUniversalSampling {
    fn select(
        &self,
        population: &Population,
        count: usize,
        rng: &mut ChaCha20Rng,
    ) -> Result<Vec<Individual>> {
        if population.is_empty() {
            return Err(EvolutionError::EmptyPopulation);
        }

        let individuals = population.individuals();

        // Calculate fitness values (shift to positive)
        let fitness_values: Vec<f64> = individuals
            .iter()
            .map(|i| i.fitness.as_ref().map(|f| f.total_score()).unwrap_or(0.0))
            .collect();

        let min_fitness = fitness_values.iter().cloned().fold(f64::INFINITY, f64::min);
        let shifted: Vec<f64> = fitness_values
            .iter()
            .map(|f| f - min_fitness + 1.0)
            .collect();

        let total: f64 = shifted.iter().sum();

        if total <= 0.0 || count == 0 {
            return Ok(Vec::new());
        }

        // Distance between pointers
        let distance = total / count as f64;

        // Random start
        let start = rng.gen::<f64>() * distance;

        // Cumulative fitness
        let mut cumulative = Vec::with_capacity(shifted.len());
        let mut sum = 0.0;
        for f in &shifted {
            sum += f;
            cumulative.push(sum);
        }

        // Select using pointers
        let mut selected = Vec::with_capacity(count);
        let mut pointer = start;
        let mut idx = 0;

        for _ in 0..count {
            while idx < cumulative.len() && cumulative[idx] < pointer {
                idx += 1;
            }
            if idx < individuals.len() {
                selected.push(individuals[idx].clone());
            }
            pointer += distance;
        }

        Ok(selected)
    }

    fn name(&self) -> &'static str {
        "StochasticUniversalSampling"
    }
}

/// Truncation selection - select top percentage
pub struct TruncationSelection {
    /// Percentage of population to select from (0.0 - 1.0)
    truncation_rate: f64,
}

impl TruncationSelection {
    pub fn new(truncation_rate: f64) -> Self {
        Self {
            truncation_rate: truncation_rate.clamp(0.1, 1.0),
        }
    }
}

impl SelectionOperator for TruncationSelection {
    fn select(
        &self,
        population: &Population,
        count: usize,
        rng: &mut ChaCha20Rng,
    ) -> Result<Vec<Individual>> {
        if population.is_empty() {
            return Err(EvolutionError::EmptyPopulation);
        }

        let mut individuals: Vec<Individual> = population.individuals().to_vec();

        // Sort by fitness
        individuals.sort_by(|a, b| {
            let fa = a
                .fitness
                .as_ref()
                .map(|f| f.total_score())
                .unwrap_or(f64::NEG_INFINITY);
            let fb = b
                .fitness
                .as_ref()
                .map(|f| f.total_score())
                .unwrap_or(f64::NEG_INFINITY);
            fb.partial_cmp(&fa).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Truncate
        let truncate_count = ((individuals.len() as f64) * self.truncation_rate) as usize;
        let truncate_count = truncate_count.max(1);
        individuals.truncate(truncate_count);

        // Randomly select from truncated pool
        let mut selected = Vec::with_capacity(count);
        for _ in 0..count {
            let idx = rng.gen_range(0..individuals.len());
            selected.push(individuals[idx].clone());
        }

        Ok(selected)
    }

    fn name(&self) -> &'static str {
        "TruncationSelection"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chromosome::Chromosome;
    use crate::fitness::FitnessScore;
    use rand::SeedableRng;

    fn test_population() -> Population {
        let mut pop = Population::new(10);

        for i in 0..10 {
            let bytecode = vec![0x20, i as u8, 0x00, 0x00];
            let chromosome = Chromosome::from_bytecode(bytecode).unwrap();
            let mut individual = Individual::new(chromosome);
            individual.fitness = Some(FitnessScore {
                pnl: i as f64 * 0.1,
                sharpe_ratio: i as f64 * 0.2,
                ..Default::default()
            });
            pop.add(individual);
        }

        pop
    }

    #[test]
    fn test_tournament_selection() {
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        let pop = test_population();
        let selection = TournamentSelection::new(3);

        let selected = selection.select(&pop, 5, &mut rng).unwrap();
        assert_eq!(selected.len(), 5);
    }

    #[test]
    fn test_roulette_selection() {
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        let pop = test_population();
        let selection = RouletteSelection::new();

        let selected = selection.select(&pop, 5, &mut rng).unwrap();
        assert_eq!(selected.len(), 5);
    }

    #[test]
    fn test_elite_selection() {
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        let pop = test_population();
        let selection = EliteSelection::new(3);

        let selected = selection.select(&pop, 3, &mut rng).unwrap();
        assert_eq!(selected.len(), 3);

        // Should be the top 3 by fitness
        for individual in &selected {
            let fitness = individual.fitness.as_ref().unwrap().pnl;
            assert!(fitness >= 0.7); // Top 3 have pnl >= 0.7
        }
    }

    #[test]
    fn test_rank_selection() {
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        let pop = test_population();
        let selection = RankSelection::new();

        let selected = selection.select(&pop, 5, &mut rng).unwrap();
        assert_eq!(selected.len(), 5);
    }

    #[test]
    fn test_sus_selection() {
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        let pop = test_population();
        let selection = StochasticUniversalSampling::new();

        let selected = selection.select(&pop, 5, &mut rng).unwrap();
        assert_eq!(selected.len(), 5);
    }

    #[test]
    fn test_truncation_selection() {
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        let pop = test_population();
        let selection = TruncationSelection::new(0.5);

        let selected = selection.select(&pop, 5, &mut rng).unwrap();
        assert_eq!(selected.len(), 5);
    }
}
