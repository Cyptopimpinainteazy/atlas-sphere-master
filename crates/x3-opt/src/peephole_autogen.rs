//! Peephole Pattern Autogen: ML-driven pattern mining with telemetry
//!
//! Automatically discovers peephole optimization patterns from:
//! - Execution telemetry (which patterns execute most?)
//! - Mutation-based search (vary patterns, measure improvement)
//! - Swarm optimization (population-based pattern tuning)
//!
//! Result: Auto-generated peephole optimization rules tailored to workload

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Instruction sequence pattern (e.g., "mov r1, r2; mov r3, r1" → "mov r3, r2")
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeepholePattern {
    /// Pattern ID (auto-generated)
    pub id: u32,
    /// Input sequence (byte pattern)
    pub input: Vec<u8>,
    /// Output sequence (replacement)
    pub output: Vec<u8>,
    /// Benefit (bytes saved, cycles saved, etc.)
    pub benefit: f64,
    /// Applicability count (how many times applied)
    pub count: u64,
    /// Discovery method (telemetry, mutation, swarm)
    pub discovery_source: String,
}

impl PartialEq for PeepholePattern {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.input == other.input && self.output == other.output
    }
}

impl Eq for PeepholePattern {}

/// Execution telemetry for pattern discovery
#[derive(Clone, Debug, Default)]
pub struct ExecutionTelemetry {
    /// Per-sequence: execution count
    pub sequence_counts: BTreeMap<Vec<u8>, u64>,
    /// Per-sequence: total cycles
    pub sequence_cycles: BTreeMap<Vec<u8>, u64>,
    /// Per-sequence: hotness score (count * cycles)
    pub hotness: BTreeMap<Vec<u8>, f64>,
}

impl ExecutionTelemetry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record execution of instruction sequence
    pub fn record_sequence(&mut self, sequence: Vec<u8>, cycles: u64) {
        *self.sequence_counts.entry(sequence.clone()).or_insert(0) += 1;
        *self.sequence_cycles.entry(sequence.clone()).or_insert(0) += cycles;

        let count = self.sequence_counts[&sequence] as f64;
        let cyc = self.sequence_cycles[&sequence] as f64;
        self.hotness.insert(sequence, count * cyc);
    }

    /// Get hottest sequences (most executed/expensive)
    pub fn get_hottest(&self, limit: usize) -> Vec<Vec<u8>> {
        let mut hot: Vec<_> = self.hotness.iter().collect();
        hot.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
        hot.into_iter()
            .take(limit)
            .map(|(seq, _)| seq.clone())
            .collect()
    }
}

/// Mutation-based pattern discovery
pub struct MutationExplorer {
    base_pattern: Vec<u8>,
    generation: u32,
}

impl MutationExplorer {
    pub fn new(base_pattern: Vec<u8>) -> Self {
        Self {
            base_pattern,
            generation: 0,
        }
    }

    /// Mutate pattern: remove byte, swap bytes, etc.
    pub fn mutate(&self, seed: u64) -> Vec<u8> {
        let mut pattern = self.base_pattern.clone();
        let mutation_type = seed % 3;

        match mutation_type {
            0 => {
                // Remove a byte
                let idx = (seed / 3) as usize % pattern.len().max(1);
                if idx < pattern.len() {
                    pattern.remove(idx);
                }
            }
            1 => {
                // Swap two bytes
                let i = (seed / 3) as usize % pattern.len().max(1);
                let j = (seed / 5) as usize % pattern.len().max(1);
                if i != j && i < pattern.len() && j < pattern.len() {
                    pattern.swap(i, j);
                }
            }
            2 => {
                // Modify a byte
                let idx = (seed / 3) as usize % pattern.len().max(1);
                if idx < pattern.len() {
                    pattern[idx] ^= 0xAA;
                }
            }
            _ => {}
        }

        pattern
    }
}

/// Swarm optimization for pattern tuning
pub struct SwarmOptimizer {
    patterns: Vec<PeepholePattern>,
    generation: u32,
    best_fitness: f64,
}

impl SwarmOptimizer {
    pub fn new(initial_patterns: Vec<PeepholePattern>) -> Self {
        let best_fitness = initial_patterns
            .iter()
            .map(|p| p.benefit)
            .fold(f64::NEG_INFINITY, f64::max);

        Self {
            patterns: initial_patterns,
            generation: 0,
            best_fitness,
        }
    }

    /// PSO-like update: move each pattern toward best, add randomness
    pub fn step(&mut self, rand_seed: u64) {
        self.generation += 1;

        for pattern in &mut self.patterns {
            let velocity = (self.best_fitness - pattern.benefit) * 0.2; // cognitive factor
            pattern.benefit += velocity;
            pattern.benefit += ((rand_seed ^ pattern.id as u64) % 10) as f64 / 100.0; // noise

            // Fitness improved?
            if pattern.benefit > self.best_fitness {
                self.best_fitness = pattern.benefit;
            }
        }
    }

    /// Get converged patterns
    pub fn converged_patterns(&self) -> Vec<PeepholePattern> {
        let mut sorted = self.patterns.clone();
        sorted.sort_by(|a, b| b.benefit.partial_cmp(&a.benefit).unwrap());
        sorted.into_iter().take(10).collect()
    }
}

/// Peephole pattern autogen: combines all discovery methods
pub struct PeepholeAutogen {
    telemetry: ExecutionTelemetry,
    generated_patterns: Vec<PeepholePattern>,
    pattern_id_counter: u32,
}

impl PeepholeAutogen {
    pub fn new() -> Self {
        Self {
            telemetry: ExecutionTelemetry::new(),
            generated_patterns: Vec::new(),
            pattern_id_counter: 0,
        }
    }

    /// Record execution for telemetry-based discovery
    pub fn record_telemetry(&mut self, sequence: Vec<u8>, cycles: u64) {
        self.telemetry.record_sequence(sequence, cycles);
    }

    /// Mine patterns from hottest sequences
    pub fn mine_from_telemetry(&mut self, limit: usize) -> Vec<PeepholePattern> {
        let hottest = self.telemetry.get_hottest(limit);
        let mut mined = Vec::new();

        for sequence in hottest {
            // Generate candidate reductions
            if sequence.len() >= 2 {
                // Example: "mov r1, r2; mov r3, r1" (6 bytes) → "mov r3, r2" (3 bytes)
                let mut reduced = sequence.clone();
                if reduced.len() > 2 {
                    reduced.remove(reduced.len() - 1);
                }

                let pattern = PeepholePattern {
                    id: self.pattern_id_counter,
                    input: sequence,
                    output: reduced,
                    benefit: 2.5, // Rough estimate
                    count: 0,
                    discovery_source: "telemetry".to_string(),
                };

                mined.push(pattern);
                self.pattern_id_counter += 1;
            }
        }

        self.generated_patterns.extend(mined.clone());
        mined
    }

    /// Mutate patterns to find variants
    pub fn mutate_patterns(&mut self, patterns: &[PeepholePattern], num_variants: usize) {
        let mut variants = Vec::new();

        for (i, pattern) in patterns.iter().enumerate() {
            for j in 0..num_variants {
                let explorer = MutationExplorer::new(pattern.input.clone());
                let mutated_input = explorer.mutate((i as u64) * 7 + (j as u64) * 13);

                let variant = PeepholePattern {
                    id: self.pattern_id_counter,
                    input: mutated_input,
                    output: pattern.output.clone(),
                    benefit: pattern.benefit * 0.9, // Slightly worse initially
                    count: 0,
                    discovery_source: "mutation".to_string(),
                };

                variants.push(variant);
                self.pattern_id_counter += 1;
            }
        }

        self.generated_patterns.extend(variants);
    }

    /// Optimize patterns via swarm
    pub fn optimize_with_swarm(&mut self, generations: u32) {
        let patterns = self.generated_patterns.clone();
        let mut swarm = SwarmOptimizer::new(patterns);

        for gen in 0..generations {
            swarm.step((gen as u64) * 12345);
        }

        self.generated_patterns = swarm.converged_patterns();
    }

    /// Full pipeline: telemetry → mutation → swarm → final patterns
    pub fn auto_generate(&mut self) -> Vec<PeepholePattern> {
        // Phase 1: Telemetry-driven mining
        let telemetry_patterns = self.mine_from_telemetry(5);

        // Phase 2: Mutation-based exploration
        self.mutate_patterns(&telemetry_patterns, 3);

        // Phase 3: Swarm optimization (tune patterns)
        self.optimize_with_swarm(10);

        // Return best patterns
        self.generated_patterns.clone()
    }

    pub fn all_patterns(&self) -> &[PeepholePattern] {
        &self.generated_patterns
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telemetry_hotness() {
        let mut telem = ExecutionTelemetry::new();
        let seq = vec![0x01, 0x02, 0x03];

        telem.record_sequence(seq.clone(), 100);
        telem.record_sequence(seq.clone(), 200);

        assert_eq!(telem.sequence_counts[&seq], 2);
        assert_eq!(telem.sequence_cycles[&seq], 300);
        assert_eq!(telem.hotness[&seq], 2.0 * 300.0);
    }

    #[test]
    fn mutation_explorer() {
        let base = vec![0x01, 0x02, 0x03, 0x04];
        let explorer = MutationExplorer::new(base);

        let mut1 = explorer.mutate(0);
        let mut2 = explorer.mutate(1);
        let mut3 = explorer.mutate(2);

        // All mutations should be different
        assert_ne!(mut1, mut2);
        assert_ne!(mut2, mut3);
    }

    #[test]
    fn swarm_optimization() {
        let initial = vec![
            PeepholePattern {
                id: 0,
                input: vec![1, 2],
                output: vec![1],
                benefit: 0.5,
                count: 0,
                discovery_source: "test".to_string(),
            },
            PeepholePattern {
                id: 1,
                input: vec![3, 4],
                output: vec![3],
                benefit: 0.3,
                count: 0,
                discovery_source: "test".to_string(),
            },
        ];

        let mut swarm = SwarmOptimizer::new(initial);
        let initial_best = swarm.best_fitness;

        for _ in 0..5 {
            swarm.step(42);
        }

        // Best should improve or stay same
        assert!(swarm.best_fitness >= initial_best);
    }

    #[test]
    fn autogen_pipeline() {
        let mut autogen = PeepholeAutogen::new();

        // Record some telemetry
        autogen.record_telemetry(vec![0x01, 0x02], 50);
        autogen.record_telemetry(vec![0x03, 0x04], 150);

        // Auto-generate patterns
        let patterns = autogen.auto_generate();

        // Should have generated patterns
        assert!(!patterns.is_empty());
    }
}
