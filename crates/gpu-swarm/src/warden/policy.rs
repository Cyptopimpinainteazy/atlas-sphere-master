//! Compute Lane Policies and Constraints
//!
//! Defines the compute lanes and their allocation policies.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Compute lanes in the GPU swarm
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComputeLane {
    /// AI Research & Model Training
    Research,
    /// AI Agents (coding, trading, analysis)
    AiAgents,
    /// Evolution Engine (EvoBot Baby Genesis) - currently disabled
    Evolution,
    /// Security & Threat Detection
    Security,
    /// Chain Morphing & Compiler Tasks
    ChainOps,
    /// dApp & Ecosystem Builders
    Ecosystem,
    /// Bot Execution (MEV, arbitrage, HFT)
    Strategy,
    /// File Storage / Filecoin-like operations
    Storage,
    /// Overflow Creators (startup bots, art bots, services)
    Overflow,
}

impl ComputeLane {
    /// Get all lanes
    pub fn all() -> Vec<Self> {
        vec![
            Self::Research,
            Self::AiAgents,
            Self::Evolution,
            Self::Security,
            Self::ChainOps,
            Self::Ecosystem,
            Self::Strategy,
            Self::Storage,
            Self::Overflow,
        ]
    }

    /// Get critical lanes that must always have minimum allocation
    pub fn critical_lanes() -> Vec<Self> {
        vec![Self::Security, Self::ChainOps]
    }

    /// Get revenue-generating lanes
    pub fn revenue_lanes() -> Vec<Self> {
        vec![Self::Strategy, Self::AiAgents, Self::Overflow]
    }

    /// Get research/growth lanes
    pub fn growth_lanes() -> Vec<Self> {
        vec![Self::Research, Self::Evolution, Self::Ecosystem]
    }

    /// Check if this lane is critical
    pub fn is_critical(&self) -> bool {
        matches!(self, Self::Security | Self::ChainOps)
    }

    /// Check if this lane generates revenue
    pub fn is_revenue(&self) -> bool {
        matches!(self, Self::Strategy | Self::AiAgents | Self::Overflow)
    }

    /// Get base priority (higher = more important)
    pub fn base_priority(&self) -> u8 {
        match self {
            Self::Security => 100, // Always highest
            Self::ChainOps => 90,  // Chain stability critical
            Self::Strategy => 80,  // Revenue generation
            Self::AiAgents => 70,  // Active agents
            Self::Research => 60,  // Long-term value
            Self::Evolution => 50, // Experimental
            Self::Ecosystem => 40, // Growth
            Self::Storage => 30,   // Utility
            Self::Overflow => 10,  // Only when spare capacity
        }
    }

    /// Get human-readable name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Research => "AI Research & Training",
            Self::AiAgents => "AI Agents",
            Self::Evolution => "Evolution Engine",
            Self::Security => "Security & Threat Detection",
            Self::ChainOps => "Chain Operations",
            Self::Ecosystem => "Ecosystem Builders",
            Self::Strategy => "Strategy & Trading Bots",
            Self::Storage => "File Storage",
            Self::Overflow => "Overflow Creators",
        }
    }
}

/// Constraints for a compute lane
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaneConstraints {
    /// Minimum allocation percentage (0.0 - 1.0)
    pub min_allocation: f64,
    /// Maximum allocation percentage (0.0 - 1.0)
    pub max_allocation: f64,
    /// Default/target allocation percentage
    pub default_allocation: f64,
    /// Whether this lane can be completely disabled
    pub can_disable: bool,
    /// Whether this lane requires GPU
    pub requires_gpu: bool,
    /// Minimum VRAM required (MB)
    pub min_vram_mb: u32,
    /// Priority boost during emergencies
    pub emergency_boost: f64,
}

impl Default for LaneConstraints {
    fn default() -> Self {
        Self {
            min_allocation: 0.0,
            max_allocation: 0.5,
            default_allocation: 0.1,
            can_disable: true,
            requires_gpu: true,
            min_vram_mb: 512,
            emergency_boost: 1.0,
        }
    }
}

/// Allocation policy for the entire swarm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationPolicy {
    /// Constraints per lane
    pub lane_constraints: HashMap<ComputeLane, LaneConstraints>,
    /// Global maximum for any single lane
    pub global_max_allocation: f64,
    /// Total must sum to this (usually 1.0)
    pub total_allocation: f64,
    /// Enable overflow lane when utilization drops below this
    pub overflow_threshold: f64,
    /// Enable evolution lane (disabled by default for stability)
    pub evolution_enabled: bool,
}

impl Default for AllocationPolicy {
    fn default() -> Self {
        let mut lane_constraints = HashMap::new();

        // Security: Always on, minimum 15%
        lane_constraints.insert(
            ComputeLane::Security,
            LaneConstraints {
                min_allocation: 0.15,
                max_allocation: 0.80, // Can surge during attacks
                default_allocation: 0.20,
                can_disable: false,
                requires_gpu: true,
                min_vram_mb: 1024,
                emergency_boost: 4.0,
            },
        );

        // Chain Ops: Always on, minimum 10%
        lane_constraints.insert(
            ComputeLane::ChainOps,
            LaneConstraints {
                min_allocation: 0.10,
                max_allocation: 0.40,
                default_allocation: 0.15,
                can_disable: false,
                requires_gpu: true,
                min_vram_mb: 2048,
                emergency_boost: 2.0,
            },
        );

        // Research: Primary value generator
        lane_constraints.insert(
            ComputeLane::Research,
            LaneConstraints {
                min_allocation: 0.05,
                max_allocation: 0.40,
                default_allocation: 0.25,
                can_disable: false,
                requires_gpu: true,
                min_vram_mb: 4096,
                emergency_boost: 0.5, // Reduced during emergencies
            },
        );

        // Strategy: Revenue generation
        lane_constraints.insert(
            ComputeLane::Strategy,
            LaneConstraints {
                min_allocation: 0.05,
                max_allocation: 0.35,
                default_allocation: 0.20,
                can_disable: false,
                requires_gpu: true,
                min_vram_mb: 2048,
                emergency_boost: 0.8,
            },
        );

        // AI Agents
        lane_constraints.insert(
            ComputeLane::AiAgents,
            LaneConstraints {
                min_allocation: 0.0,
                max_allocation: 0.25,
                default_allocation: 0.10,
                can_disable: true,
                requires_gpu: true,
                min_vram_mb: 1024,
                emergency_boost: 0.5,
            },
        );

        // Ecosystem
        lane_constraints.insert(
            ComputeLane::Ecosystem,
            LaneConstraints {
                min_allocation: 0.0,
                max_allocation: 0.20,
                default_allocation: 0.05,
                can_disable: true,
                requires_gpu: true,
                min_vram_mb: 512,
                emergency_boost: 0.3,
            },
        );

        // Storage
        lane_constraints.insert(
            ComputeLane::Storage,
            LaneConstraints {
                min_allocation: 0.0,
                max_allocation: 0.15,
                default_allocation: 0.03,
                can_disable: true,
                requires_gpu: false,
                min_vram_mb: 0,
                emergency_boost: 0.2,
            },
        );

        // Evolution: Disabled by default
        lane_constraints.insert(
            ComputeLane::Evolution,
            LaneConstraints {
                min_allocation: 0.0,
                max_allocation: 0.15,
                default_allocation: 0.0, // Disabled
                can_disable: true,
                requires_gpu: true,
                min_vram_mb: 2048,
                emergency_boost: 0.0, // Never runs during emergencies
            },
        );

        // Overflow: Only when capacity available
        lane_constraints.insert(
            ComputeLane::Overflow,
            LaneConstraints {
                min_allocation: 0.0,
                max_allocation: 0.20,
                default_allocation: 0.02,
                can_disable: true,
                requires_gpu: true,
                min_vram_mb: 512,
                emergency_boost: 0.0, // Never runs during emergencies
            },
        );

        Self {
            lane_constraints,
            global_max_allocation: 0.50,
            total_allocation: 1.0,
            overflow_threshold: 0.60, // Enable overflow when <60% utilized
            evolution_enabled: false, // Disabled until main swarm stable
        }
    }
}

impl AllocationPolicy {
    /// Get constraints for a lane
    pub fn get_constraints(&self, lane: ComputeLane) -> LaneConstraints {
        self.lane_constraints
            .get(&lane)
            .cloned()
            .unwrap_or_default()
    }

    /// Validate that allocations sum to 1.0 and respect constraints
    pub fn validate_allocations(&self, allocations: &HashMap<ComputeLane, f64>) -> Vec<String> {
        let mut errors = Vec::new();

        // Check sum
        let total: f64 = allocations.values().sum();
        if (total - self.total_allocation).abs() > 0.01 {
            errors.push(format!(
                "Allocations sum to {:.2}, expected {:.2}",
                total, self.total_allocation
            ));
        }

        // Check per-lane constraints
        for (lane, alloc) in allocations {
            if let Some(constraints) = self.lane_constraints.get(lane) {
                if *alloc < constraints.min_allocation {
                    errors.push(format!(
                        "{:?} allocation {:.2}% below minimum {:.2}%",
                        lane,
                        alloc * 100.0,
                        constraints.min_allocation * 100.0
                    ));
                }
                if *alloc > constraints.max_allocation {
                    errors.push(format!(
                        "{:?} allocation {:.2}% above maximum {:.2}%",
                        lane,
                        alloc * 100.0,
                        constraints.max_allocation * 100.0
                    ));
                }
            }
        }

        // Check global max
        for (lane, alloc) in allocations {
            if *alloc > self.global_max_allocation {
                errors.push(format!(
                    "{:?} exceeds global max allocation of {:.2}%",
                    lane,
                    self.global_max_allocation * 100.0
                ));
            }
        }

        errors
    }

    /// Get default allocations
    pub fn default_allocations(&self) -> HashMap<ComputeLane, f64> {
        self.lane_constraints
            .iter()
            .map(|(lane, c)| (*lane, c.default_allocation))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_lane_priority() {
        assert!(ComputeLane::Security.base_priority() > ComputeLane::Overflow.base_priority());
        assert!(ComputeLane::ChainOps.base_priority() > ComputeLane::Research.base_priority());
    }

    #[test]
    fn test_lane_constraints_default() {
        let policy = AllocationPolicy::default();

        // Security must have min allocation
        let security = policy.get_constraints(ComputeLane::Security);
        assert!(security.min_allocation >= 0.15);
        assert!(!security.can_disable);

        // Evolution disabled by default
        let evolution = policy.get_constraints(ComputeLane::Evolution);
        assert_eq!(evolution.default_allocation, 0.0);
    }

    #[test]
    fn test_allocation_validation() {
        let policy = AllocationPolicy::default();

        // Valid allocation
        let mut allocations = HashMap::new();
        allocations.insert(ComputeLane::Security, 0.20);
        allocations.insert(ComputeLane::ChainOps, 0.15);
        allocations.insert(ComputeLane::Research, 0.25);
        allocations.insert(ComputeLane::Strategy, 0.20);
        allocations.insert(ComputeLane::AiAgents, 0.10);
        allocations.insert(ComputeLane::Ecosystem, 0.05);
        allocations.insert(ComputeLane::Storage, 0.03);
        allocations.insert(ComputeLane::Overflow, 0.02);

        let errors = policy.validate_allocations(&allocations);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_default_allocations_sum() {
        let policy = AllocationPolicy::default();
        let defaults = policy.default_allocations();
        let sum: f64 = defaults.values().sum();

        // Should sum to 1.0 (allowing small float error)
        assert!(
            (sum - 1.0).abs() < 0.05,
            "Default allocations sum to {}, expected ~1.0",
            sum
        );
    }
}
