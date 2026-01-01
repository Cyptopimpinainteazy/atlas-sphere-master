//! Combatant - a strategy competing in the arena
//!
//! Tracks:
//! - Strategy genome
//! - Elo rating
//! - Win/loss record
//! - Status (active, eliminated)

use crate::evolution::Genome;
use crate::types::StrategyId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Combatant status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CombatantStatus {
    /// Ready to compete
    Active,
    /// Temporarily suspended
    Suspended,
    /// Permanently eliminated
    Eliminated,
    /// Being evaluated
    InMatch,
}

/// A strategy competing in the arena
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Combatant {
    /// Strategy ID
    pub id: StrategyId,
    /// Strategy genome
    pub genome: Genome,
    /// Current status
    pub status: CombatantStatus,
    /// Elo rating
    pub elo: f64,
    /// Total wins
    pub wins: usize,
    /// Total losses
    pub losses: usize,
    /// Total draws
    pub draws: usize,
    /// Win streak
    pub win_streak: usize,
    /// Highest Elo achieved
    pub peak_elo: f64,
    /// Total profit/loss (simulated)
    pub total_pnl: f64,
    /// Best single match score
    pub best_score: f64,
    /// When registered
    pub registered_at: DateTime<Utc>,
    /// When eliminated (if applicable)
    pub eliminated_at: Option<DateTime<Utc>>,
}

impl Combatant {
    /// Create new combatant from genome
    pub fn new(genome: Genome) -> Self {
        Self {
            id: genome.id,
            genome,
            status: CombatantStatus::Active,
            elo: 1200.0, // Starting Elo
            wins: 0,
            losses: 0,
            draws: 0,
            win_streak: 0,
            peak_elo: 1200.0,
            total_pnl: 0.0,
            best_score: 0.0,
            registered_at: Utc::now(),
            eliminated_at: None,
        }
    }

    /// Check if combatant is active
    pub fn is_active(&self) -> bool {
        self.status == CombatantStatus::Active
    }

    /// Record a win
    pub fn record_win(&mut self, score: f64) {
        self.wins += 1;
        self.win_streak += 1;
        self.total_pnl += score;
        self.best_score = self.best_score.max(score);

        // Update Elo (simplified K-factor)
        let k = self.k_factor();
        self.elo += k * 0.5;
        self.peak_elo = self.peak_elo.max(self.elo);
    }

    /// Record a loss
    pub fn record_loss(&mut self, score: f64) {
        self.losses += 1;
        self.win_streak = 0;
        self.total_pnl += score;

        // Update Elo
        let k = self.k_factor();
        self.elo -= k * 0.5;
        self.elo = self.elo.max(100.0); // Floor
    }

    /// Record a draw
    pub fn record_draw(&mut self, score: f64) {
        self.draws += 1;
        self.total_pnl += score;
        // Elo unchanged in draw
    }

    /// Calculate K-factor (higher for newer/volatile players)
    fn k_factor(&self) -> f64 {
        let games = self.wins + self.losses + self.draws;
        if games < 30 {
            40.0 // New player
        } else if self.elo > 2400.0 {
            16.0 // Expert
        } else {
            32.0 // Normal
        }
    }

    /// Get win rate
    pub fn win_rate(&self) -> f64 {
        let total = self.wins + self.losses;
        if total == 0 {
            0.0
        } else {
            self.wins as f64 / total as f64
        }
    }

    /// Eliminate combatant
    pub fn eliminate(&mut self) {
        self.status = CombatantStatus::Eliminated;
        self.eliminated_at = Some(Utc::now());
    }

    /// Suspend combatant
    pub fn suspend(&mut self) {
        self.status = CombatantStatus::Suspended;
    }

    /// Reactivate combatant
    pub fn reactivate(&mut self) {
        if self.status == CombatantStatus::Suspended {
            self.status = CombatantStatus::Active;
        }
    }

    /// Mark as in match
    pub fn enter_match(&mut self) {
        if self.status == CombatantStatus::Active {
            self.status = CombatantStatus::InMatch;
        }
    }

    /// Exit match
    pub fn exit_match(&mut self) {
        if self.status == CombatantStatus::InMatch {
            self.status = CombatantStatus::Active;
        }
    }

    /// Get rank based on Elo
    pub fn rank_title(&self) -> &'static str {
        match self.elo as i64 {
            0..=999 => "Bronze",
            1000..=1199 => "Silver",
            1200..=1399 => "Gold",
            1400..=1599 => "Platinum",
            1600..=1799 => "Diamond",
            1800..=1999 => "Master",
            2000..=2199 => "Grandmaster",
            2200..=2399 => "Champion",
            _ => "Legend",
        }
    }

    /// Performance score (composite metric)
    pub fn performance_score(&self) -> f64 {
        let elo_component = (self.elo - 1200.0) / 800.0; // Normalized around 0
        let win_rate_component = self.win_rate() * 2.0 - 1.0; // -1 to 1
        let pnl_component = (self.total_pnl / 1000.0).clamp(-1.0, 1.0);

        (elo_component + win_rate_component + pnl_component) / 3.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_combatant_creation() {
        let genome = Genome::new_float(5, 0.0, 1.0);
        let combatant = Combatant::new(genome);

        assert!(combatant.is_active());
        assert_eq!(combatant.elo, 1200.0);
        assert_eq!(combatant.wins, 0);
        assert_eq!(combatant.losses, 0);
    }

    #[test]
    fn test_win_loss_recording() {
        let genome = Genome::new_float(5, 0.0, 1.0);
        let mut combatant = Combatant::new(genome);

        combatant.record_win(100.0);
        assert_eq!(combatant.wins, 1);
        assert!(combatant.elo > 1200.0);
        assert_eq!(combatant.win_streak, 1);

        combatant.record_loss(50.0);
        assert_eq!(combatant.losses, 1);
        assert_eq!(combatant.win_streak, 0);
    }

    #[test]
    fn test_win_rate() {
        let genome = Genome::new_float(5, 0.0, 1.0);
        let mut combatant = Combatant::new(genome);

        combatant.wins = 7;
        combatant.losses = 3;

        assert!((combatant.win_rate() - 0.7).abs() < 0.001);
    }

    #[test]
    fn test_elimination() {
        let genome = Genome::new_float(5, 0.0, 1.0);
        let mut combatant = Combatant::new(genome);

        combatant.eliminate();

        assert!(!combatant.is_active());
        assert_eq!(combatant.status, CombatantStatus::Eliminated);
        assert!(combatant.eliminated_at.is_some());
    }

    #[test]
    fn test_rank_titles() {
        let genome = Genome::new_float(5, 0.0, 1.0);
        let mut combatant = Combatant::new(genome);

        combatant.elo = 800.0;
        assert_eq!(combatant.rank_title(), "Bronze");

        combatant.elo = 1500.0;
        assert_eq!(combatant.rank_title(), "Platinum");

        combatant.elo = 2500.0;
        assert_eq!(combatant.rank_title(), "Legend");
    }
}
