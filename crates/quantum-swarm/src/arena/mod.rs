//! Arena module for strategy competition
//!
//! Implements tournament-style competition where strategies compete,
//! losers are eliminated, and winners evolve to produce next generation.
//!
//! Core mechanics:
//! - Round-robin or bracket-style tournaments
//! - Kill threshold (bottom 10% eliminated)
//! - Winner promotion and breeding
//! - Archive system for eliminated strategies

mod archive;
mod combatant;
mod scheduler;
mod tournament;

pub use archive::{Archive, ArchivedStrategy};
pub use combatant::{Combatant, CombatantStatus};
pub use scheduler::{ArenaScheduler, ScheduledMatch};
pub use tournament::{MatchResult, Tournament, TournamentResult, TournamentType};

use crate::error::{SwarmError, SwarmResult};
use crate::evolution::{EvolutionConfig, EvolutionEngine, Genome};
use crate::types::{StrategyId, TournamentId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Arena configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArenaConfig {
    /// Maximum strategies in arena
    pub max_strategies: usize,
    /// Kill threshold (bottom X% eliminated)
    pub kill_threshold: f64,
    /// Minimum strategies to maintain
    pub min_strategies: usize,
    /// Tournament type
    pub tournament_type: TournamentType,
    /// Number of rounds per tournament
    pub rounds_per_tournament: usize,
    /// Enable archiving of eliminated strategies
    pub enable_archive: bool,
    /// Archive capacity
    pub archive_capacity: usize,
    /// Evolution config for breeding
    pub evolution_config: EvolutionConfig,
}

impl Default for ArenaConfig {
    fn default() -> Self {
        Self {
            max_strategies: 1000,
            kill_threshold: 0.10, // Bottom 10% killed
            min_strategies: 50,
            tournament_type: TournamentType::RoundRobin,
            rounds_per_tournament: 3,
            enable_archive: true,
            archive_capacity: 10000,
            evolution_config: EvolutionConfig::default(),
        }
    }
}

/// The Arena - where strategies fight for survival
pub struct Arena {
    /// Configuration
    config: ArenaConfig,
    /// Active combatants
    combatants: HashMap<StrategyId, Combatant>,
    /// Tournament scheduler
    scheduler: ArenaScheduler,
    /// Archive of eliminated strategies
    archive: Archive,
    /// Evolution engine for breeding
    evolution: EvolutionEngine,
    /// Total tournaments run
    tournaments_completed: u64,
    /// Total eliminations
    total_eliminations: u64,
}

impl Arena {
    /// Create new arena
    pub fn new(config: ArenaConfig) -> Self {
        let evolution = EvolutionEngine::new(config.evolution_config.clone());
        Self {
            scheduler: ArenaScheduler::new(config.rounds_per_tournament),
            archive: Archive::new(config.archive_capacity),
            evolution,
            config,
            combatants: HashMap::new(),
            tournaments_completed: 0,
            total_eliminations: 0,
        }
    }

    /// Register a strategy as a combatant
    pub fn register(&mut self, genome: Genome) -> SwarmResult<StrategyId> {
        if self.combatants.len() >= self.config.max_strategies {
            return Err(SwarmError::ArenaFull {
                max: self.config.max_strategies,
            });
        }

        let id = genome.id;
        let combatant = Combatant::new(genome);
        self.combatants.insert(id, combatant);

        Ok(id)
    }

    /// Run a tournament
    pub fn run_tournament(&mut self) -> SwarmResult<TournamentResult> {
        let tournament_id = TournamentId::new_v4();

        // Get active combatant IDs and count
        let active_ids: Vec<_> = self
            .combatants
            .iter()
            .filter(|(_, c)| c.is_active())
            .map(|(id, _)| *id)
            .collect();
        let participants = active_ids.len();

        if participants < 2 {
            return Err(SwarmError::NotEnoughCombatants {
                required: 2,
                available: participants,
            });
        }

        // Get references for scheduling
        let active: Vec<_> = active_ids
            .iter()
            .filter_map(|id| self.combatants.get(id))
            .collect();

        // Schedule matches based on tournament type
        let matches = match self.config.tournament_type {
            TournamentType::RoundRobin => self.scheduler.schedule_round_robin(&active),
            TournamentType::SingleElimination => {
                self.scheduler.schedule_single_elimination(&active)
            }
            TournamentType::DoubleElimination => {
                self.scheduler.schedule_double_elimination(&active)
            }
            TournamentType::Swiss { rounds } => self.scheduler.schedule_swiss(&active, rounds),
        };

        // Execute matches and collect results
        let mut match_results = Vec::new();
        for scheduled in &matches {
            let result = self.execute_match(scheduled)?;
            match_results.push(result);
        }

        // Update combatant statistics
        for result in &match_results {
            if let Some(c) = self.combatants.get_mut(&result.winner) {
                c.record_win(result.winner_score);
            }
            if let Some(c) = self.combatants.get_mut(&result.loser) {
                c.record_loss(result.loser_score);
            }
        }

        // Calculate rankings
        let mut rankings: Vec<_> = self.combatants.values().filter(|c| c.is_active()).collect();
        rankings.sort_by(|a, b| b.elo.partial_cmp(&a.elo).unwrap());

        let ranking_map: HashMap<_, _> = rankings
            .iter()
            .enumerate()
            .map(|(i, c)| (c.id, i + 1))
            .collect();

        // Eliminate bottom performers
        let eliminations = self.eliminate_losers();

        // Breed winners to maintain population
        let births = self.breed_winners(eliminations)?;

        self.tournaments_completed += 1;

        Ok(TournamentResult {
            tournament_id,
            tournament_type: self.config.tournament_type.clone(),
            participants,
            matches: match_results,
            rankings: ranking_map,
            eliminations,
            new_births: births,
        })
    }

    /// Execute a single match
    fn execute_match(&self, scheduled: &ScheduledMatch) -> SwarmResult<MatchResult> {
        let combatant_a =
            self.combatants
                .get(&scheduled.combatant_a)
                .ok_or(SwarmError::CombatantNotFound {
                    id: scheduled.combatant_a,
                })?;
        let combatant_b =
            self.combatants
                .get(&scheduled.combatant_b)
                .ok_or(SwarmError::CombatantNotFound {
                    id: scheduled.combatant_b,
                })?;

        // Score based on fitness (in real system, would run simulation)
        let score_a = combatant_a.genome.fitness + (rand::random::<f64>() - 0.5) * 0.1;
        let score_b = combatant_b.genome.fitness + (rand::random::<f64>() - 0.5) * 0.1;

        let (winner, loser, winner_score, loser_score) = if score_a > score_b {
            (
                scheduled.combatant_a,
                scheduled.combatant_b,
                score_a,
                score_b,
            )
        } else {
            (
                scheduled.combatant_b,
                scheduled.combatant_a,
                score_b,
                score_a,
            )
        };

        // Calculate Elo change
        let expected_a = 1.0 / (1.0 + 10f64.powf((combatant_b.elo - combatant_a.elo) / 400.0));
        let k = 32.0;
        let elo_change = k
            * (if winner == scheduled.combatant_a {
                1.0
            } else {
                0.0
            } - expected_a);

        Ok(MatchResult {
            match_id: uuid::Uuid::new_v4(),
            combatant_a: scheduled.combatant_a,
            combatant_b: scheduled.combatant_b,
            winner,
            loser,
            winner_score,
            loser_score,
            elo_change: elo_change.abs(),
        })
    }

    /// Eliminate bottom performers
    fn eliminate_losers(&mut self) -> usize {
        let active_count = self.combatants.values().filter(|c| c.is_active()).count();
        let kill_count = ((active_count as f64 * self.config.kill_threshold) as usize)
            .max(0)
            .min(active_count.saturating_sub(self.config.min_strategies));

        if kill_count == 0 {
            return 0;
        }

        // Sort by Elo (lowest first)
        let mut active: Vec<_> = self.combatants.values().filter(|c| c.is_active()).collect();
        active.sort_by(|a, b| a.elo.partial_cmp(&b.elo).unwrap());

        // Mark bottom for elimination
        let to_kill: Vec<_> = active.iter().take(kill_count).map(|c| c.id).collect();

        for id in &to_kill {
            if let Some(combatant) = self.combatants.get_mut(id) {
                combatant.eliminate();

                // Archive if enabled
                if self.config.enable_archive {
                    self.archive.add(ArchivedStrategy {
                        genome: combatant.genome.clone(),
                        final_elo: combatant.elo,
                        total_matches: combatant.wins + combatant.losses,
                        win_rate: combatant.win_rate(),
                        elimination_reason: "kill_threshold".to_string(),
                        archived_at: chrono::Utc::now(),
                    });
                }
            }
        }

        self.total_eliminations += to_kill.len() as u64;
        to_kill.len()
    }

    /// Breed winners to replenish population
    fn breed_winners(&mut self, target_births: usize) -> SwarmResult<usize> {
        if target_births == 0 {
            return Ok(0);
        }

        // Select top performers as parents - clone their genomes to avoid borrow issues
        let mut winners: Vec<_> = self
            .combatants
            .values()
            .filter(|c| c.is_active())
            .map(|c| (c.elo, c.genome.clone()))
            .collect();
        winners.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

        let parent_pool_size = (winners.len() / 2).max(2);
        let parents: Vec<Genome> = winners
            .into_iter()
            .take(parent_pool_size)
            .map(|(_, g)| g)
            .collect();

        let mut births = 0;
        for _ in 0..target_births {
            if parents.len() < 2 {
                break;
            }

            // Select two random parents
            let p1_idx = rand::random::<usize>() % parents.len();
            let p2_idx =
                (p1_idx + 1 + rand::random::<usize>() % (parents.len() - 1)) % parents.len();

            let parent1 = &parents[p1_idx];
            let parent2 = &parents[p2_idx];

            // Crossover
            let mut child = parent1.crossover(parent2);

            // Mutate
            child.mutate(&self.config.evolution_config.mutation);

            // Register child
            if self.register(child).is_ok() {
                births += 1;
            }
        }

        Ok(births)
    }

    /// Get combatant by ID
    pub fn get_combatant(&self, id: StrategyId) -> Option<&Combatant> {
        self.combatants.get(&id)
    }

    /// Get all active combatants
    pub fn active_combatants(&self) -> Vec<&Combatant> {
        self.combatants.values().filter(|c| c.is_active()).collect()
    }

    /// Get leaderboard
    pub fn leaderboard(&self, top_n: usize) -> Vec<&Combatant> {
        let mut active: Vec<_> = self.combatants.values().filter(|c| c.is_active()).collect();
        active.sort_by(|a, b| b.elo.partial_cmp(&a.elo).unwrap());
        active.into_iter().take(top_n).collect()
    }

    /// Get archive
    pub fn archive(&self) -> &Archive {
        &self.archive
    }

    /// Get statistics
    pub fn stats(&self) -> ArenaStats {
        let active_count = self.combatants.values().filter(|c| c.is_active()).count();
        let total_matches: usize = self
            .combatants
            .values()
            .map(|c| c.wins + c.losses)
            .sum::<usize>()
            / 2; // Each match counted twice

        let avg_elo = if active_count > 0 {
            self.combatants
                .values()
                .filter(|c| c.is_active())
                .map(|c| c.elo)
                .sum::<f64>()
                / active_count as f64
        } else {
            1200.0
        };

        ArenaStats {
            active_combatants: active_count,
            total_combatants: self.combatants.len(),
            tournaments_completed: self.tournaments_completed,
            total_eliminations: self.total_eliminations,
            total_matches,
            average_elo: avg_elo,
            archived_strategies: self.archive.len(),
        }
    }
}

/// Arena statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArenaStats {
    pub active_combatants: usize,
    pub total_combatants: usize,
    pub tournaments_completed: u64,
    pub total_eliminations: u64,
    pub total_matches: usize,
    pub average_elo: f64,
    pub archived_strategies: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evolution::Genome;

    #[test]
    fn test_arena_creation() {
        let config = ArenaConfig::default();
        let arena = Arena::new(config);

        assert_eq!(arena.combatants.len(), 0);
        assert_eq!(arena.tournaments_completed, 0);
    }

    #[test]
    fn test_register_combatant() {
        let config = ArenaConfig::default();
        let mut arena = Arena::new(config);

        let genome = Genome::new_float(5, 0.0, 1.0);
        let id = arena.register(genome).unwrap();

        assert!(arena.combatants.contains_key(&id));
    }

    #[test]
    fn test_arena_tournament() {
        let config = ArenaConfig {
            kill_threshold: 0.0, // Don't kill for this test
            ..Default::default()
        };
        let mut arena = Arena::new(config);

        // Register several combatants
        for i in 0..10 {
            let mut genome = Genome::new_float(5, 0.0, 1.0);
            genome.fitness = i as f64 * 0.1;
            arena.register(genome).unwrap();
        }

        let result = arena.run_tournament().unwrap();

        assert!(result.matches.len() > 0);
        assert_eq!(result.participants, 10);
    }
}
