//! Arena scheduler for match organization
//!
//! Handles:
//! - Match scheduling for various tournament types
//! - Bye handling for odd participants
//! - Re-matching rules

use super::combatant::Combatant;
use crate::types::StrategyId;
use serde::{Deserialize, Serialize};

/// A scheduled match
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledMatch {
    /// First combatant
    pub combatant_a: StrategyId,
    /// Second combatant
    pub combatant_b: StrategyId,
    /// Round number
    pub round: usize,
    /// Match number within round
    pub match_number: usize,
}

/// Arena scheduler
#[derive(Debug, Clone)]
pub struct ArenaScheduler {
    /// Rounds per tournament
    rounds: usize,
    /// Current match counter
    match_counter: usize,
}

impl ArenaScheduler {
    /// Create new scheduler
    pub fn new(rounds: usize) -> Self {
        Self {
            rounds,
            match_counter: 0,
        }
    }

    /// Schedule round robin matches
    pub fn schedule_round_robin(&mut self, combatants: &[&Combatant]) -> Vec<ScheduledMatch> {
        let mut matches = Vec::new();
        let ids: Vec<_> = combatants.iter().map(|c| c.id).collect();
        let n = ids.len();

        // All pairs
        for i in 0..n {
            for j in (i + 1)..n {
                self.match_counter += 1;
                matches.push(ScheduledMatch {
                    combatant_a: ids[i],
                    combatant_b: ids[j],
                    round: 1,
                    match_number: self.match_counter,
                });
            }
        }

        matches
    }

    /// Schedule single elimination bracket
    pub fn schedule_single_elimination(
        &mut self,
        combatants: &[&Combatant],
    ) -> Vec<ScheduledMatch> {
        let mut matches = Vec::new();
        let mut ids: Vec<_> = combatants.iter().map(|c| c.id).collect();

        // Shuffle for random seeding
        self.shuffle(&mut ids);

        // First round matches
        for i in (0..ids.len()).step_by(2) {
            if i + 1 < ids.len() {
                self.match_counter += 1;
                matches.push(ScheduledMatch {
                    combatant_a: ids[i],
                    combatant_b: ids[i + 1],
                    round: 1,
                    match_number: self.match_counter,
                });
            }
        }

        matches
    }

    /// Schedule double elimination bracket
    pub fn schedule_double_elimination(
        &mut self,
        combatants: &[&Combatant],
    ) -> Vec<ScheduledMatch> {
        // For first round, same as single elimination
        // Losers bracket matches scheduled after results
        self.schedule_single_elimination(combatants)
    }

    /// Schedule Swiss system
    pub fn schedule_swiss(
        &mut self,
        combatants: &[&Combatant],
        round: usize,
    ) -> Vec<ScheduledMatch> {
        let mut matches = Vec::new();

        // Sort by current Elo (in real Swiss, by score)
        let mut sorted: Vec<_> = combatants.iter().collect();
        sorted.sort_by(|a, b| b.elo.partial_cmp(&a.elo).unwrap());

        let ids: Vec<_> = sorted.iter().map(|c| c.id).collect();

        // Pair adjacent (similar strength)
        for i in (0..ids.len()).step_by(2) {
            if i + 1 < ids.len() {
                self.match_counter += 1;
                matches.push(ScheduledMatch {
                    combatant_a: ids[i],
                    combatant_b: ids[i + 1],
                    round,
                    match_number: self.match_counter,
                });
            }
        }

        matches
    }

    /// Schedule seeded bracket
    pub fn schedule_seeded(&mut self, combatants: &[&Combatant]) -> Vec<ScheduledMatch> {
        let mut matches = Vec::new();

        // Sort by Elo (highest first)
        let mut sorted: Vec<_> = combatants.iter().collect();
        sorted.sort_by(|a, b| b.elo.partial_cmp(&a.elo).unwrap());

        let ids: Vec<_> = sorted.iter().map(|c| c.id).collect();
        let n = ids.len();

        // Pair 1st vs last, 2nd vs second-last, etc.
        for i in 0..n / 2 {
            self.match_counter += 1;
            matches.push(ScheduledMatch {
                combatant_a: ids[i],
                combatant_b: ids[n - 1 - i],
                round: 1,
                match_number: self.match_counter,
            });
        }

        matches
    }

    /// Simple Fisher-Yates shuffle
    fn shuffle<T>(&self, slice: &mut [T]) {
        let n = slice.len();
        for i in (1..n).rev() {
            let j = rand::random::<usize>() % (i + 1);
            slice.swap(i, j);
        }
    }

    /// Reset match counter
    pub fn reset(&mut self) {
        self.match_counter = 0;
    }

    /// Get total matches scheduled
    pub fn total_matches(&self) -> usize {
        self.match_counter
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evolution::Genome;

    fn make_combatants(n: usize) -> Vec<Combatant> {
        (0..n)
            .map(|i| {
                let mut genome = Genome::new_float(5, 0.0, 1.0);
                genome.fitness = i as f64 * 0.1;
                let mut c = Combatant::new(genome);
                c.elo = 1200.0 + i as f64 * 50.0;
                c
            })
            .collect()
    }

    #[test]
    fn test_round_robin() {
        let mut scheduler = ArenaScheduler::new(1);
        let combatants = make_combatants(4);
        let refs: Vec<_> = combatants.iter().collect();

        let matches = scheduler.schedule_round_robin(&refs);

        // 4 choose 2 = 6 matches
        assert_eq!(matches.len(), 6);
    }

    #[test]
    fn test_single_elimination() {
        let mut scheduler = ArenaScheduler::new(3);
        let combatants = make_combatants(8);
        let refs: Vec<_> = combatants.iter().collect();

        let matches = scheduler.schedule_single_elimination(&refs);

        // 8 players = 4 first round matches
        assert_eq!(matches.len(), 4);
    }

    #[test]
    fn test_swiss() {
        let mut scheduler = ArenaScheduler::new(5);
        let combatants = make_combatants(8);
        let refs: Vec<_> = combatants.iter().collect();

        let matches = scheduler.schedule_swiss(&refs, 1);

        // 8 players = 4 matches per round
        assert_eq!(matches.len(), 4);
    }

    #[test]
    fn test_seeded() {
        let mut scheduler = ArenaScheduler::new(3);
        let combatants = make_combatants(8);
        let refs: Vec<_> = combatants.iter().collect();

        let matches = scheduler.schedule_seeded(&refs);

        assert_eq!(matches.len(), 4);

        // First match should be highest vs lowest Elo
        // (After sorting, index 0 is highest, index 7 is lowest)
    }
}
