//! GPU Allocator
//!
//! Dynamic GPU allocation engine that distributes resources across compute lanes.

use crate::warden::policy::{AllocationPolicy, ComputeLane, LaneConstraints};
use crate::warden::signals::{LaneMetrics, SignalAggregator};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Allocation for a single lane
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaneAllocation {
    /// Lane being allocated
    pub lane: ComputeLane,
    /// Percentage of total compute (0.0 - 1.0)
    pub compute_percent: f64,
    /// Number of GPU units allocated
    pub gpu_units: u32,
    /// VRAM allocated (MB)
    pub vram_mb: u32,
    /// Priority ranking (1 = highest)
    pub priority_rank: u8,
    /// Reason for this allocation
    pub reason: String,
}

/// Complete allocation plan for the swarm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationPlan {
    /// Allocations per lane
    pub allocations: HashMap<ComputeLane, LaneAllocation>,
    /// Total GPU units available
    pub total_gpu_units: u32,
    /// Total VRAM available (MB)
    pub total_vram_mb: u64,
    /// Confidence in this plan (0.0 - 1.0)
    pub confidence: f64,
    /// Whether this is an emergency reallocation
    pub is_emergency: bool,
    /// Plan generation timestamp
    pub generated_at_ms: u64,
    /// Changes from previous plan
    pub changes: Vec<AllocationChange>,
}

impl AllocationPlan {
    /// Validate plan respects constraints
    pub fn validate(&self, policy: &AllocationPolicy) -> Vec<String> {
        let percent_map: HashMap<ComputeLane, f64> = self
            .allocations
            .iter()
            .map(|(l, a)| (*l, a.compute_percent))
            .collect();

        policy.validate_allocations(&percent_map)
    }

    /// Get allocation for a lane
    pub fn get(&self, lane: ComputeLane) -> Option<&LaneAllocation> {
        self.allocations.get(&lane)
    }

    /// Calculate utilization (allocated / total)
    pub fn utilization(&self) -> f64 {
        let allocated: f64 = self.allocations.values().map(|a| a.compute_percent).sum();
        allocated / 1.0 // Total is always 1.0
    }
}

/// A change in allocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationChange {
    pub lane: ComputeLane,
    pub from_percent: f64,
    pub to_percent: f64,
    pub reason: String,
}

impl AllocationChange {
    pub fn delta(&self) -> f64 {
        self.to_percent - self.from_percent
    }

    pub fn is_increase(&self) -> bool {
        self.to_percent > self.from_percent
    }
}

/// GPU Allocator - computes optimal allocation across lanes
pub struct GpuAllocator {
    /// Allocation policy
    policy: AllocationPolicy,
    /// Previous allocation plan
    previous_plan: Option<AllocationPlan>,
    /// Minimum change threshold (avoid thrashing)
    min_change_threshold: f64,
    /// Emergency multiplier for security lane
    emergency_multiplier: f64,
}

impl Default for GpuAllocator {
    fn default() -> Self {
        Self::new(AllocationPolicy::default())
    }
}

impl GpuAllocator {
    /// Create new allocator with policy
    pub fn new(policy: AllocationPolicy) -> Self {
        Self {
            policy,
            previous_plan: None,
            min_change_threshold: 0.02, // 2% minimum change
            emergency_multiplier: 2.0,
        }
    }

    /// Compute optimal allocation based on current signals
    pub fn compute_allocation(
        &mut self,
        aggregator: &SignalAggregator,
        total_gpu_units: u32,
        total_vram_mb: u64,
        is_emergency: bool,
    ) -> AllocationPlan {
        let mut allocations = HashMap::new();
        let mut changes = Vec::new();

        // Step 1: Calculate base allocations from policy defaults
        let mut target_allocations = self.policy.default_allocations();

        // Step 2: Adjust based on lane metrics
        for lane in ComputeLane::all() {
            if let Some(metrics) = aggregator.get_metrics(lane) {
                let adjustment = self.calculate_adjustment(lane, metrics, is_emergency);
                if let Some(current) = target_allocations.get_mut(&lane) {
                    *current = (*current + adjustment).clamp(0.0, 1.0);
                }
            }
        }

        // Step 3: Apply emergency mode if needed
        if is_emergency {
            self.apply_emergency_mode(&mut target_allocations);
        }

        // Step 4: Normalize to sum to 1.0 while respecting constraints
        self.normalize_allocations(&mut target_allocations);

        // Step 5: Apply minimum change threshold (anti-thrashing)
        if let Some(prev) = &self.previous_plan {
            self.apply_change_threshold(&mut target_allocations, prev);
        }

        // Step 6: Build allocation plan
        let mut priority_rank = 1u8;
        for lane in self.sorted_by_priority(&target_allocations) {
            let percent = target_allocations.get(&lane).copied().unwrap_or(0.0);
            let constraints = self.policy.get_constraints(lane);

            let gpu_units = (total_gpu_units as f64 * percent).round() as u32;
            let vram_mb = if constraints.requires_gpu {
                ((total_vram_mb as f64 * percent).round() as u32).max(constraints.min_vram_mb)
            } else {
                0
            };

            // Track changes
            if let Some(prev) = &self.previous_plan {
                if let Some(prev_alloc) = prev.allocations.get(&lane) {
                    let delta = (percent - prev_alloc.compute_percent).abs();
                    if delta > 0.001 {
                        changes.push(AllocationChange {
                            lane,
                            from_percent: prev_alloc.compute_percent,
                            to_percent: percent,
                            reason: self.change_reason(lane, aggregator.get_metrics(lane)),
                        });
                    }
                }
            }

            let reason = self.allocation_reason(lane, percent, is_emergency);
            allocations.insert(
                lane,
                LaneAllocation {
                    lane,
                    compute_percent: percent,
                    gpu_units,
                    vram_mb,
                    priority_rank,
                    reason,
                },
            );

            priority_rank += 1;
        }

        let plan = AllocationPlan {
            allocations,
            total_gpu_units,
            total_vram_mb,
            confidence: self.calculate_confidence(aggregator, is_emergency),
            is_emergency,
            generated_at_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            changes,
        };

        self.previous_plan = Some(plan.clone());
        plan
    }

    /// Calculate adjustment for a lane based on metrics
    fn calculate_adjustment(
        &self,
        lane: ComputeLane,
        metrics: &LaneMetrics,
        is_emergency: bool,
    ) -> f64 {
        let constraints = self.policy.get_constraints(lane);

        // Base adjustment from urgency
        let urgency = metrics.urgency_score();
        let mut adjustment = (urgency - 0.5) * 0.1; // ±5% base adjustment

        // Starved lanes get boost
        if metrics.is_starved() {
            adjustment += 0.05;
        }

        // Excess capacity lanes get reduced
        if metrics.has_excess_capacity() {
            adjustment -= 0.03;
        }

        // Revenue-generating lanes with high throughput get boost
        if lane.is_revenue() && metrics.avg_throughput > 10.0 {
            adjustment += 0.02;
        }

        // Apply emergency boost/reduction
        if is_emergency {
            adjustment *= constraints.emergency_boost;
        }

        // Critical lanes always get positive adjustment
        if lane.is_critical() && adjustment < 0.0 {
            adjustment = 0.0;
        }

        adjustment
    }

    /// Apply emergency mode allocations
    fn apply_emergency_mode(&self, allocations: &mut HashMap<ComputeLane, f64>) {
        // Security gets emergency boost
        if let Some(security) = allocations.get_mut(&ComputeLane::Security) {
            *security = (*security * self.emergency_multiplier).min(0.6);
        }

        // Non-critical lanes get reduced
        for lane in [
            ComputeLane::Overflow,
            ComputeLane::Evolution,
            ComputeLane::Ecosystem,
            ComputeLane::Storage,
        ] {
            if let Some(alloc) = allocations.get_mut(&lane) {
                *alloc *= 0.2; // 80% reduction
            }
        }
    }

    /// Normalize allocations to sum to 1.0 while respecting constraints
    fn normalize_allocations(&self, allocations: &mut HashMap<ComputeLane, f64>) {
        // First pass: enforce minimums
        for (lane, alloc) in allocations.iter_mut() {
            let constraints = self.policy.get_constraints(*lane);
            if *alloc < constraints.min_allocation {
                *alloc = constraints.min_allocation;
            }
        }

        // Second pass: enforce maximums
        for (lane, alloc) in allocations.iter_mut() {
            let constraints = self.policy.get_constraints(*lane);
            if *alloc > constraints.max_allocation {
                *alloc = constraints.max_allocation;
            }
            if *alloc > self.policy.global_max_allocation {
                *alloc = self.policy.global_max_allocation;
            }
        }

        // Third pass: scale to sum to 1.0
        let sum: f64 = allocations.values().sum();
        if sum > 0.0 {
            let scale = 1.0 / sum;
            for alloc in allocations.values_mut() {
                *alloc *= scale;
            }
        }

        // Final pass: re-enforce minimums (scaling might have reduced them)
        let mut overflow = 0.0;
        for (lane, alloc) in allocations.iter_mut() {
            let constraints = self.policy.get_constraints(*lane);
            if *alloc < constraints.min_allocation {
                overflow += constraints.min_allocation - *alloc;
                *alloc = constraints.min_allocation;
            }
        }

        // Distribute overflow by reducing non-critical lanes
        if overflow > 0.0 {
            let reducible: Vec<ComputeLane> = allocations
                .iter()
                .filter(|(l, _)| !l.is_critical())
                .map(|(l, _)| *l)
                .collect();

            let reduce_each = overflow / reducible.len().max(1) as f64;
            for lane in reducible {
                if let Some(alloc) = allocations.get_mut(&lane) {
                    let constraints = self.policy.get_constraints(lane);
                    *alloc = (*alloc - reduce_each).max(constraints.min_allocation);
                }
            }
        }
    }

    /// Apply minimum change threshold to prevent thrashing
    fn apply_change_threshold(
        &self,
        allocations: &mut HashMap<ComputeLane, f64>,
        previous: &AllocationPlan,
    ) {
        for (lane, alloc) in allocations.iter_mut() {
            if let Some(prev_alloc) = previous.allocations.get(lane) {
                let delta = *alloc - prev_alloc.compute_percent;
                if delta.abs() < self.min_change_threshold {
                    *alloc = prev_alloc.compute_percent;
                }
            }
        }
    }

    /// Sort lanes by priority
    fn sorted_by_priority(&self, allocations: &HashMap<ComputeLane, f64>) -> Vec<ComputeLane> {
        let mut lanes: Vec<_> = allocations.keys().copied().collect();
        lanes.sort_by(|a, b| b.base_priority().cmp(&a.base_priority()));
        lanes
    }

    /// Generate reason for allocation
    fn allocation_reason(&self, lane: ComputeLane, percent: f64, is_emergency: bool) -> String {
        if is_emergency && lane == ComputeLane::Security {
            return "Emergency security response".to_string();
        }

        let constraints = self.policy.get_constraints(lane);
        if percent <= constraints.min_allocation {
            format!(
                "Minimum allocation ({:.0}%)",
                constraints.min_allocation * 100.0
            )
        } else if percent >= constraints.max_allocation {
            format!(
                "Maximum allocation ({:.0}%)",
                constraints.max_allocation * 100.0
            )
        } else {
            format!("Optimized for current load ({:.1}%)", percent * 100.0)
        }
    }

    /// Generate change reason
    fn change_reason(&self, lane: ComputeLane, metrics: Option<&LaneMetrics>) -> String {
        match metrics {
            Some(m) if m.is_starved() => "Lane starved, increasing allocation".to_string(),
            Some(m) if m.has_excess_capacity() => "Excess capacity, redistributing".to_string(),
            Some(m) if m.error_rate > 0.1 => "High error rate, adjusting".to_string(),
            _ => format!("{} allocation rebalanced", lane.display_name()),
        }
    }

    /// Calculate confidence in allocation plan
    fn calculate_confidence(&self, aggregator: &SignalAggregator, is_emergency: bool) -> f64 {
        let health = aggregator.swarm_health();
        let starved = aggregator.starved_lanes().len();
        let excess = aggregator.excess_lanes().len();

        let mut confidence = health;

        // Reduce confidence if many lanes are imbalanced
        if starved > 2 {
            confidence *= 0.8;
        }
        if excess > 3 {
            confidence *= 0.9;
        }

        // Emergency mode reduces confidence
        if is_emergency {
            confidence *= 0.7;
        }

        confidence.clamp(0.0, 1.0)
    }

    /// Get current policy
    pub fn policy(&self) -> &AllocationPolicy {
        &self.policy
    }

    /// Update policy
    pub fn set_policy(&mut self, policy: AllocationPolicy) {
        self.policy = policy;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_basic_allocation() {
        let mut allocator = GpuAllocator::default();
        let aggregator = SignalAggregator::new(Duration::from_secs(60), 100);

        let plan = allocator.compute_allocation(&aggregator, 100, 102400, false);

        // Should have allocations for all lanes
        assert!(!plan.allocations.is_empty());

        // Security should have minimum allocation
        let security = plan.get(ComputeLane::Security).unwrap();
        assert!(security.compute_percent >= 0.15);

        // Total should sum to ~1.0
        let total: f64 = plan.allocations.values().map(|a| a.compute_percent).sum();
        assert!((total - 1.0).abs() < 0.05, "Total: {}", total);
    }

    #[test]
    fn test_emergency_mode() {
        let mut allocator = GpuAllocator::default();
        let aggregator = SignalAggregator::new(Duration::from_secs(60), 100);

        let normal_plan = allocator.compute_allocation(&aggregator, 100, 102400, false);
        let emergency_plan = allocator.compute_allocation(&aggregator, 100, 102400, true);

        // Security should be higher in emergency
        let normal_security = normal_plan.get(ComputeLane::Security).unwrap();
        let emergency_security = emergency_plan.get(ComputeLane::Security).unwrap();

        assert!(
            emergency_security.compute_percent >= normal_security.compute_percent,
            "Emergency security: {}, Normal: {}",
            emergency_security.compute_percent,
            normal_security.compute_percent
        );

        assert!(emergency_plan.is_emergency);
    }

    #[test]
    fn test_constraint_enforcement() {
        let mut allocator = GpuAllocator::default();
        let aggregator = SignalAggregator::new(Duration::from_secs(60), 100);

        let plan = allocator.compute_allocation(&aggregator, 100, 102400, false);
        let errors = plan.validate(allocator.policy());

        // With default policy, plan should be valid
        assert!(errors.is_empty(), "Validation errors: {:?}", errors);
    }
}
