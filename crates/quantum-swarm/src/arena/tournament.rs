//! Tournament types and execution
//!
//! Supports multiple tournament formats:
//! - Round robin (all vs all)
//! - Single elimination (bracket)
//! - Double elimination
//! - Swiss system

use crate::types::{StrategyId, TournamentId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Tournament type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TournamentType {
    /// Every combatant faces every other
    RoundRobin,
    /// Single elimination bracket
    SingleElimination,
    /// Double elimination (losers bracket)
    DoubleElimination,
    /// Swiss system with fixed rounds
    Swiss { rounds: usize },
}

impl Default for TournamentType {
    fn default() -> Self {
        Self::RoundRobin
    }
}

/// Result of a single match
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResult {
    /// Match identifier
    pub match_id: uuid::Uuid,
    /// First combatant
    pub combatant_a: StrategyId,
    /// Second combatant
    pub combatant_b: StrategyId,
    /// Winner
    pub winner: StrategyId,
    /// Loser
    pub loser: StrategyId,
    /// Winner's score
    pub winner_score: f64,
    /// Loser's score
    pub loser_score: f64,
    /// Elo rating change
    pub elo_change: f64,
}

/// Complete tournament results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TournamentResult {
    /// Tournament ID
    pub tournament_id: TournamentId,
    /// Tournament type
    pub tournament_type: TournamentType,
    /// Number of participants
    pub participants: usize,
    /// All match results
    pub matches: Vec<MatchResult>,
    /// Final rankings (strategy -> rank)
    pub rankings: HashMap<StrategyId, usize>,
    /// Number of eliminations
    pub eliminations: usize,
    /// New strategies born from breeding
    pub new_births: usize,
}

impl TournamentResult {
    /// Get champion (rank 1)
    pub fn champion(&self) -> Option<StrategyId> {
        self.rankings
            .iter()
            .find(|(_, &rank)| rank == 1)
            .map(|(&id, _)| id)
    }

    /// Get top N strategies
    pub fn top_n(&self, n: usize) -> Vec<StrategyId> {
        let mut ranked: Vec<_> = self.rankings.iter().collect();
        ranked.sort_by_key(|(_, rank)| *rank);
        ranked.into_iter().take(n).map(|(&id, _)| id).collect()
    }

    /// Get win count for strategy
    pub fn wins(&self, id: StrategyId) -> usize {
        self.matches.iter().filter(|m| m.winner == id).count()
    }

    /// Get loss count for strategy
    pub fn losses(&self, id: StrategyId) -> usize {
        self.matches.iter().filter(|m| m.loser == id).count()
    }
}

/// Tournament state machine
#[derive(Debug, Clone)]
pub struct Tournament {
    /// Tournament ID
    pub id: TournamentId,
    /// Tournament type
    pub tournament_type: TournamentType,
    /// Current round
    pub current_round: usize,
    /// Total rounds
    pub total_rounds: usize,
    /// Matches in current round
    pub current_matches: Vec<PendingMatch>,
    /// Completed matches
    pub completed_matches: Vec<MatchResult>,
    /// Active combatants (for elimination)
    pub active: Vec<StrategyId>,
    /// Eliminated (for double elim)
    pub losers_bracket: Vec<StrategyId>,
    /// Is tournament complete
    pub complete: bool,
}

/// A pending match to be executed
#[derive(Debug, Clone)]
pub struct PendingMatch {
    pub combatant_a: StrategyId,
    pub combatant_b: StrategyId,
    pub round: usize,
    pub bracket: Bracket,
}

/// Bracket type for elimination tournaments
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bracket {
    Winners,
    Losers,
    Finals,
}

impl Tournament {
    /// Create new tournament
    pub fn new(tournament_type: TournamentType, participants: Vec<StrategyId>) -> Self {
        let total_rounds = match &tournament_type {
            TournamentType::RoundRobin => 1,
            TournamentType::SingleElimination => (participants.len() as f64).log2().ceil() as usize,
            TournamentType::DoubleElimination => {
                ((participants.len() as f64).log2().ceil() as usize) * 2 - 1
            }
            TournamentType::Swiss { rounds } => *rounds,
        };

        Self {
            id: TournamentId::new_v4(),
            tournament_type,
            current_round: 0,
            total_rounds,
            current_matches: Vec::new(),
            completed_matches: Vec::new(),
            active: participants,
            losers_bracket: Vec::new(),
            complete: false,
        }
    }

    /// Check if tournament is finished
    pub fn is_complete(&self) -> bool {
        self.complete
    }

    /// Get next round of matches
    pub fn next_round(&mut self) -> Vec<PendingMatch> {
        if self.complete || self.current_round >= self.total_rounds {
            self.complete = true;
            return Vec::new();
        }

        self.current_round += 1;

        match &self.tournament_type {
            TournamentType::RoundRobin => self.generate_round_robin(),
            TournamentType::SingleElimination => self.generate_single_elimination(),
            TournamentType::DoubleElimination => self.generate_double_elimination(),
            TournamentType::Swiss { .. } => self.generate_swiss(),
        }
    }

    /// Generate round robin pairings
    fn generate_round_robin(&mut self) -> Vec<PendingMatch> {
        let mut matches = Vec::new();
        let n = self.active.len();

        for i in 0..n {
            for j in (i + 1)..n {
                matches.push(PendingMatch {
                    combatant_a: self.active[i],
                    combatant_b: self.active[j],
                    round: self.current_round,
                    bracket: Bracket::Winners,
                });
            }
        }

        self.complete = true; // Round robin is single round
        self.current_matches = matches.clone();
        matches
    }

    /// Generate single elimination bracket
    fn generate_single_elimination(&mut self) -> Vec<PendingMatch> {
        let mut matches = Vec::new();

        // Pair adjacent combatants
        for i in (0..self.active.len()).step_by(2) {
            if i + 1 < self.active.len() {
                matches.push(PendingMatch {
                    combatant_a: self.active[i],
                    combatant_b: self.active[i + 1],
                    round: self.current_round,
                    bracket: Bracket::Winners,
                });
            }
        }

        self.current_matches = matches.clone();
        matches
    }

    /// Generate double elimination bracket
    fn generate_double_elimination(&mut self) -> Vec<PendingMatch> {
        let mut matches = Vec::new();

        // Winners bracket
        for i in (0..self.active.len()).step_by(2) {
            if i + 1 < self.active.len() {
                matches.push(PendingMatch {
                    combatant_a: self.active[i],
                    combatant_b: self.active[i + 1],
                    round: self.current_round,
                    bracket: Bracket::Winners,
                });
            }
        }

        // Losers bracket
        for i in (0..self.losers_bracket.len()).step_by(2) {
            if i + 1 < self.losers_bracket.len() {
                matches.push(PendingMatch {
                    combatant_a: self.losers_bracket[i],
                    combatant_b: self.losers_bracket[i + 1],
                    round: self.current_round,
                    bracket: Bracket::Losers,
                });
            }
        }

        self.current_matches = matches.clone();
        matches
    }

    /// Generate Swiss pairings
    fn generate_swiss(&mut self) -> Vec<PendingMatch> {
        // In Swiss, pair combatants with similar scores
        // For now, use simple adjacent pairing
        let mut matches = Vec::new();

        for i in (0..self.active.len()).step_by(2) {
            if i + 1 < self.active.len() {
                matches.push(PendingMatch {
                    combatant_a: self.active[i],
                    combatant_b: self.active[i + 1],
                    round: self.current_round,
                    bracket: Bracket::Winners,
                });
            }
        }

        self.current_matches = matches.clone();
        matches
    }

    /// Record match result
    pub fn record_result(&mut self, result: MatchResult) {
        self.completed_matches.push(result.clone());

        // Update brackets based on tournament type
        match self.tournament_type {
            TournamentType::SingleElimination => {
                // Loser is out
                self.active.retain(|&id| id != result.loser);
                if self.active.len() <= 1 {
                    self.complete = true;
                }
            }
            TournamentType::DoubleElimination => {
                // Move loser to losers bracket
                if !self.losers_bracket.contains(&result.loser) {
                    self.losers_bracket.push(result.loser);
                    self.active.retain(|&id| id != result.loser);
                } else {
                    // Already in losers, now out
                    self.losers_bracket.retain(|&id| id != result.loser);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ids(n: usize) -> Vec<StrategyId> {
        (0..n).map(|_| StrategyId::new_v4()).collect()
    }

    #[test]
    fn test_round_robin() {
        let participants = make_ids(4);
        let mut tournament = Tournament::new(TournamentType::RoundRobin, participants.clone());

        let matches = tournament.next_round();

        // 4 players = 6 matches (4 choose 2)
        assert_eq!(matches.len(), 6);
        assert!(tournament.is_complete());
    }

    #[test]
    fn test_single_elimination() {
        let participants = make_ids(8);
        let mut tournament =
            Tournament::new(TournamentType::SingleElimination, participants.clone());

        let round1 = tournament.next_round();
        assert_eq!(round1.len(), 4);
        assert!(!tournament.is_complete());
    }

    #[test]
    fn test_swiss() {
        let participants = make_ids(8);
        let mut tournament =
            Tournament::new(TournamentType::Swiss { rounds: 3 }, participants.clone());

        let round1 = tournament.next_round();
        assert_eq!(round1.len(), 4);
        assert!(!tournament.is_complete());
    }

    #[test]
    fn test_tournament_result() {
        let id1 = StrategyId::new_v4();
        let id2 = StrategyId::new_v4();

        let result = TournamentResult {
            tournament_id: TournamentId::new_v4(),
            tournament_type: TournamentType::RoundRobin,
            participants: 2,
            matches: vec![MatchResult {
                match_id: uuid::Uuid::new_v4(),
                combatant_a: id1,
                combatant_b: id2,
                winner: id1,
                loser: id2,
                winner_score: 1.0,
                loser_score: 0.5,
                elo_change: 16.0,
            }],
            rankings: vec![(id1, 1), (id2, 2)].into_iter().collect(),
            eliminations: 0,
            new_births: 0,
        };

        assert_eq!(result.champion(), Some(id1));
        assert_eq!(result.wins(id1), 1);
        assert_eq!(result.losses(id2), 1);
    }
}
