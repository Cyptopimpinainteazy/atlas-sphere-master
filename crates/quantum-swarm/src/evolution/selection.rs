//! Selection operators for evolutionary algorithms
//!
//! Implements various selection strategies:
//! - Tournament selection
//! - Roulette wheel (fitness proportionate)
//! - Rank-based selection
//! - NSGA-II (multi-objective)

use serde::{Deserialize, Serialize};

/// Selection operator type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SelectionOperator {
    /// Tournament selection
    Tournament {
        /// Tournament size
        size: usize,
    },
    /// Roulette wheel (fitness proportionate)
    RouletteWheel,
    /// Rank-based selection
    Rank,
    /// NSGA-II for multi-objective optimization
    Nsga2,
}

impl Default for SelectionOperator {
    fn default() -> Self {
        Self::Tournament { size: 5 }
    }
}

impl SelectionOperator {
    /// Create tournament selection
    pub fn tournament(size: usize) -> Self {
        Self::Tournament { size: size.max(2) }
    }

    /// Create roulette wheel selection
    pub fn roulette() -> Self {
        Self::RouletteWheel
    }

    /// Create rank selection
    pub fn rank() -> Self {
        Self::Rank
    }

    /// Create NSGA-II selection
    pub fn nsga2() -> Self {
        Self::Nsga2
    }
}

/// Selection statistics
#[derive(Debug, Default, Clone)]
pub struct Selection {
    /// Total selections made
    pub total_selections: usize,
    /// Average fitness of selected
    pub avg_selected_fitness: f64,
    /// Selection pressure (ratio of best to avg)
    pub selection_pressure: f64,
}

impl Selection {
    /// Create new selection tracker
    pub fn new() -> Self {
        Self::default()
    }

    /// Record selection event
    pub fn record(&mut self, selected_fitness: f64, population_avg: f64) {
        self.total_selections += 1;

        // Running average
        let n = self.total_selections as f64;
        self.avg_selected_fitness =
            self.avg_selected_fitness * ((n - 1.0) / n) + selected_fitness / n;

        if population_avg > 0.0 {
            self.selection_pressure = self.avg_selected_fitness / population_avg;
        }
    }

    /// Reset statistics
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Pareto front for multi-objective optimization
#[derive(Debug, Clone)]
pub struct ParetoFront {
    /// Individuals on the front (indices)
    pub individuals: Vec<usize>,
    /// Crowding distances
    pub crowding_distances: Vec<f64>,
}

impl ParetoFront {
    /// Create new Pareto front
    pub fn new() -> Self {
        Self {
            individuals: Vec::new(),
            crowding_distances: Vec::new(),
        }
    }

    /// Calculate non-dominated sorting
    pub fn non_dominated_sort(objectives: &[Vec<f64>]) -> Vec<ParetoFront> {
        let n = objectives.len();
        if n == 0 {
            return vec![];
        }

        let mut domination_count = vec![0usize; n];
        let mut dominated_by: Vec<Vec<usize>> = vec![vec![]; n];
        let mut fronts = vec![];
        let mut first_front = vec![];

        // Calculate domination relationships
        for i in 0..n {
            for j in 0..n {
                if i == j {
                    continue;
                }

                if dominates(&objectives[i], &objectives[j]) {
                    dominated_by[i].push(j);
                } else if dominates(&objectives[j], &objectives[i]) {
                    domination_count[i] += 1;
                }
            }

            if domination_count[i] == 0 {
                first_front.push(i);
            }
        }

        // Build fronts
        let mut current_front = first_front;
        while !current_front.is_empty() {
            let mut front = ParetoFront::new();
            front.individuals = current_front.clone();
            front.crowding_distances = calculate_crowding_distance(&current_front, objectives);
            fronts.push(front);

            let mut next_front = vec![];
            for &i in &current_front {
                for &j in &dominated_by[i] {
                    domination_count[j] -= 1;
                    if domination_count[j] == 0 {
                        next_front.push(j);
                    }
                }
            }
            current_front = next_front;
        }

        fronts
    }
}

impl Default for ParetoFront {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if solution a dominates solution b
fn dominates(a: &[f64], b: &[f64]) -> bool {
    let mut dominated = false;
    for (ai, bi) in a.iter().zip(b.iter()) {
        if ai < bi {
            return false;
        }
        if ai > bi {
            dominated = true;
        }
    }
    dominated
}

/// Calculate crowding distances
fn calculate_crowding_distance(front: &[usize], objectives: &[Vec<f64>]) -> Vec<f64> {
    let n = front.len();
    if n == 0 {
        return vec![];
    }
    if n <= 2 {
        return vec![f64::INFINITY; n];
    }

    let num_objectives = objectives[front[0]].len();
    let mut distances = vec![0.0; n];

    for m in 0..num_objectives {
        // Sort by objective m
        let mut indices: Vec<usize> = (0..n).collect();
        indices.sort_by(|&i, &j| {
            objectives[front[i]][m]
                .partial_cmp(&objectives[front[j]][m])
                .unwrap()
        });

        // Boundary points get infinite distance
        distances[indices[0]] = f64::INFINITY;
        distances[indices[n - 1]] = f64::INFINITY;

        // Calculate range
        let obj_min = objectives[front[indices[0]]][m];
        let obj_max = objectives[front[indices[n - 1]]][m];
        let range = obj_max - obj_min;

        if range > 0.0 {
            for i in 1..(n - 1) {
                let prev = objectives[front[indices[i - 1]]][m];
                let next = objectives[front[indices[i + 1]]][m];
                distances[indices[i]] += (next - prev) / range;
            }
        }
    }

    distances
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection_operators() {
        let tournament = SelectionOperator::tournament(5);
        if let SelectionOperator::Tournament { size } = tournament {
            assert_eq!(size, 5);
        }

        let roulette = SelectionOperator::roulette();
        assert!(matches!(roulette, SelectionOperator::RouletteWheel));
    }

    #[test]
    fn test_domination() {
        let a = vec![1.0, 2.0];
        let b = vec![0.5, 1.5];
        let c = vec![1.5, 1.5];

        assert!(dominates(&a, &b)); // a dominates b
        assert!(!dominates(&b, &a)); // b doesn't dominate a
        assert!(!dominates(&a, &c)); // a doesn't dominate c (c is better in second obj)
    }

    #[test]
    fn test_pareto_front() {
        let objectives = vec![
            vec![1.0, 1.0], // Non-dominated
            vec![0.5, 2.0], // Non-dominated
            vec![0.3, 0.3], // Dominated
            vec![2.0, 0.5], // Non-dominated
        ];

        let fronts = ParetoFront::non_dominated_sort(&objectives);
        assert!(!fronts.is_empty());

        // First front should have the non-dominated solutions
        let first_front = &fronts[0];
        assert!(first_front.individuals.contains(&0));
        assert!(first_front.individuals.contains(&1));
        assert!(first_front.individuals.contains(&3));
    }

    #[test]
    fn test_selection_tracking() {
        let mut tracker = Selection::new();

        tracker.record(0.8, 0.5);
        tracker.record(0.9, 0.5);

        assert_eq!(tracker.total_selections, 2);
        assert!((tracker.avg_selected_fitness - 0.85).abs() < 0.01);
    }
}
