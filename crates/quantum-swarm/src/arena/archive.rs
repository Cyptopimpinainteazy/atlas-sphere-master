//! Archive for eliminated strategies
//!
//! Preserves strategies after elimination for:
//! - Analysis and learning
//! - Potential resurrection
//! - Historical tracking

use crate::evolution::Genome;
use crate::types::StrategyId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// An archived strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivedStrategy {
    /// The strategy genome
    pub genome: Genome,
    /// Final Elo rating
    pub final_elo: f64,
    /// Total matches played
    pub total_matches: usize,
    /// Win rate
    pub win_rate: f64,
    /// Reason for elimination
    pub elimination_reason: String,
    /// When archived
    pub archived_at: DateTime<Utc>,
}

impl ArchivedStrategy {
    /// Get strategy ID
    pub fn id(&self) -> StrategyId {
        self.genome.id
    }

    /// Check if strategy was successful (high Elo)
    pub fn was_successful(&self) -> bool {
        self.final_elo > 1400.0 && self.win_rate > 0.5
    }
}

/// Archive of eliminated strategies
#[derive(Debug)]
pub struct Archive {
    /// Archived strategies (FIFO with capacity)
    strategies: VecDeque<ArchivedStrategy>,
    /// Maximum capacity
    capacity: usize,
    /// Total strategies ever archived
    total_archived: u64,
}

impl Archive {
    /// Create new archive
    pub fn new(capacity: usize) -> Self {
        Self {
            strategies: VecDeque::with_capacity(capacity),
            capacity,
            total_archived: 0,
        }
    }

    /// Add strategy to archive
    pub fn add(&mut self, strategy: ArchivedStrategy) {
        if self.strategies.len() >= self.capacity {
            self.strategies.pop_front();
        }
        self.strategies.push_back(strategy);
        self.total_archived += 1;
    }

    /// Get strategy by ID
    pub fn get(&self, id: StrategyId) -> Option<&ArchivedStrategy> {
        self.strategies.iter().find(|s| s.genome.id == id)
    }

    /// Get all archived strategies
    pub fn all(&self) -> impl Iterator<Item = &ArchivedStrategy> {
        self.strategies.iter()
    }

    /// Get successful strategies (for potential resurrection)
    pub fn successful(&self) -> Vec<&ArchivedStrategy> {
        self.strategies
            .iter()
            .filter(|s| s.was_successful())
            .collect()
    }

    /// Get top N by Elo
    pub fn top_by_elo(&self, n: usize) -> Vec<&ArchivedStrategy> {
        let mut sorted: Vec<_> = self.strategies.iter().collect();
        sorted.sort_by(|a, b| b.final_elo.partial_cmp(&a.final_elo).unwrap());
        sorted.into_iter().take(n).collect()
    }

    /// Get by elimination reason
    pub fn by_reason(&self, reason: &str) -> Vec<&ArchivedStrategy> {
        self.strategies
            .iter()
            .filter(|s| s.elimination_reason.contains(reason))
            .collect()
    }

    /// Get recent archives
    pub fn recent(&self, n: usize) -> Vec<&ArchivedStrategy> {
        self.strategies.iter().rev().take(n).collect()
    }

    /// Get number of archived strategies
    pub fn len(&self) -> usize {
        self.strategies.len()
    }

    /// Check if archive is empty
    pub fn is_empty(&self) -> bool {
        self.strategies.is_empty()
    }

    /// Get total ever archived
    pub fn total_archived(&self) -> u64 {
        self.total_archived
    }

    /// Clear archive
    pub fn clear(&mut self) {
        self.strategies.clear();
    }

    /// Get archive statistics
    pub fn stats(&self) -> ArchiveStats {
        if self.strategies.is_empty() {
            return ArchiveStats::default();
        }

        let total = self.strategies.len();
        let successful = self.successful().len();
        let elo_sum: f64 = self.strategies.iter().map(|s| s.final_elo).sum();
        let win_rate_sum: f64 = self.strategies.iter().map(|s| s.win_rate).sum();

        ArchiveStats {
            total_archived: self.total_archived,
            current_count: total,
            successful_count: successful,
            avg_final_elo: elo_sum / total as f64,
            avg_win_rate: win_rate_sum / total as f64,
        }
    }
}

impl Clone for Archive {
    fn clone(&self) -> Self {
        Self {
            strategies: self.strategies.clone(),
            capacity: self.capacity,
            total_archived: self.total_archived,
        }
    }
}

/// Archive statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArchiveStats {
    pub total_archived: u64,
    pub current_count: usize,
    pub successful_count: usize,
    pub avg_final_elo: f64,
    pub avg_win_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evolution::Genome;

    #[test]
    fn test_archive_creation() {
        let archive = Archive::new(100);
        assert!(archive.is_empty());
        assert_eq!(archive.total_archived(), 0);
    }

    #[test]
    fn test_archive_add() {
        let mut archive = Archive::new(100);

        let strategy = ArchivedStrategy {
            genome: Genome::new_float(5, 0.0, 1.0),
            final_elo: 1500.0,
            total_matches: 50,
            win_rate: 0.6,
            elimination_reason: "kill_threshold".to_string(),
            archived_at: Utc::now(),
        };

        let id = strategy.genome.id;
        archive.add(strategy);

        assert_eq!(archive.len(), 1);
        assert!(archive.get(id).is_some());
    }

    #[test]
    fn test_archive_capacity() {
        let mut archive = Archive::new(3);

        for i in 0..5 {
            let mut genome = Genome::new_float(5, 0.0, 1.0);
            genome.fitness = i as f64;

            archive.add(ArchivedStrategy {
                genome,
                final_elo: 1200.0 + i as f64 * 100.0,
                total_matches: 10,
                win_rate: 0.5,
                elimination_reason: "test".to_string(),
                archived_at: Utc::now(),
            });
        }

        // Should only keep last 3
        assert_eq!(archive.len(), 3);
        assert_eq!(archive.total_archived(), 5);
    }

    #[test]
    fn test_successful_filter() {
        let mut archive = Archive::new(100);

        // Successful strategy
        let mut genome1 = Genome::new_float(5, 0.0, 1.0);
        genome1.fitness = 1.0;
        archive.add(ArchivedStrategy {
            genome: genome1,
            final_elo: 1600.0,
            total_matches: 100,
            win_rate: 0.65,
            elimination_reason: "test".to_string(),
            archived_at: Utc::now(),
        });

        // Unsuccessful strategy
        let mut genome2 = Genome::new_float(5, 0.0, 1.0);
        genome2.fitness = 0.2;
        archive.add(ArchivedStrategy {
            genome: genome2,
            final_elo: 1100.0,
            total_matches: 50,
            win_rate: 0.3,
            elimination_reason: "test".to_string(),
            archived_at: Utc::now(),
        });

        let successful = archive.successful();
        assert_eq!(successful.len(), 1);
    }
}
