//! The Scrapyard Pipeline - The Forge of Failure
//!
//! When any agent, bot, contract, program, or mutation starts behaving weird,
//! unprofitable, misaligned, high-risk, inefficient, exploitable, unstable,
//! or outright cursed... it gets sent to the Scrapyard.
//!
//! The Scrapyard doesn't destroy first - it LEARNS first.
//!
//! Pipeline:
//! 1. QUARANTINE - Sandbox, study, understand
//! 2. DISASSEMBLY - Parse logic, identify innovations, detect exploits
//! 3. RECYCLING OR EXECUTION - Extract useful parts or destroy
//! 4. MEMORY INJECTION - Feed knowledge back to the swarm
//!
//! "90% of genius comes from failed attempts"

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Maximum modules in quarantine
pub const MAX_QUARANTINE_SIZE: usize = 100;

/// Default quarantine duration (24 hours)
pub const DEFAULT_QUARANTINE_DURATION: Duration = Duration::from_secs(86400);

/// Minimum observation time before disassembly
pub const MIN_OBSERVATION_TIME: Duration = Duration::from_secs(3600);

/// Why a module was sent to the scrapyard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuarantineReason {
    /// Module behaving erratically
    Erratic { description: String },
    /// Module is unprofitable
    Unprofitable { loss_amount: f64, period: String },
    /// Module is misaligned with goals
    Misaligned { expected: String, actual: String },
    /// Module poses high risk
    HighRisk { risk_factors: Vec<String> },
    /// Module is inefficient
    Inefficient { efficiency: f64, threshold: f64 },
    /// Module has exploitable vulnerabilities
    Exploitable { vulnerability: String },
    /// Module is unstable
    Unstable { crash_count: u32, error_types: Vec<String> },
    /// Module attempting to game the system
    Gaming { strategy: String },
    /// Module producing dangerous outputs
    Dangerous { threat_type: String },
    /// Module runaway resource consumption
    Runaway { resource: String, consumption: f64 },
    /// Manual flagging by Crown or admin
    ManualFlag { reason: String, flagged_by: String },
}

/// Current state of a module in the scrapyard
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScrapyardStage {
    /// Initial quarantine and observation
    Quarantine,
    /// Being disassembled and analyzed
    Disassembly,
    /// Pending verdict
    PendingVerdict,
    /// Being recycled for parts
    Recycling,
    /// Scheduled for destruction
    Execution,
    /// Process complete
    Complete,
}

/// Verdict on what to do with module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScrapyardVerdict {
    /// Recycle useful parts
    Recycle {
        useful_parts: Vec<RecyclablePart>,
        justification: String,
    },
    /// Destroy completely
    Execute {
        reason: String,
        blacklist: bool,
    },
    /// Return to service (false positive)
    Rehabilitate {
        conditions: Vec<String>,
        probation_period: Duration,
    },
    /// Needs more observation
    ExtendObservation {
        duration: Duration,
        focus_areas: Vec<String>,
    },
}

/// A recyclable part extracted from a module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecyclablePart {
    /// Part identifier
    pub id: String,
    /// Type of knowledge
    pub part_type: RecyclableType,
    /// The actual content/knowledge
    pub content: String,
    /// Confidence in usefulness
    pub confidence: f64,
    /// Suggested targets for injection
    pub inject_targets: Vec<String>,
}

/// Types of recyclable knowledge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecyclableType {
    /// Novel algorithm or heuristic
    Algorithm,
    /// Useful pattern discovered
    Pattern,
    /// Optimization technique
    Optimization,
    /// Security insight
    SecurityInsight,
    /// Trading strategy fragment
    StrategyFragment,
    /// Model weights (partial)
    ModelWeights,
    /// Training data
    TrainingData,
    /// Error handling technique
    ErrorHandling,
    /// Communication pattern
    NetworkPattern,
    /// Resource management trick
    ResourceManagement,
}

/// Module in the scrapyard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapyardModule {
    /// Module identifier
    pub module_id: String,
    /// Original module type
    pub module_type: String,
    /// Why it's here
    pub reason: QuarantineReason,
    /// Current stage
    pub stage: ScrapyardStage,
    /// When quarantined
    pub quarantined_at: u64,
    /// Observations made during quarantine
    pub observations: Vec<Observation>,
    /// Disassembly report (if available)
    pub disassembly: Option<DisassemblyReport>,
    /// Final verdict (if decided)
    pub verdict: Option<ScrapyardVerdict>,
    /// Tags for categorization
    pub tags: Vec<String>,
}

/// Observation during quarantine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub timestamp: u64,
    pub observer: String,
    pub observation_type: ObservationType,
    pub content: String,
    pub significance: f64,
}

/// Types of observations
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObservationType {
    /// Behavioral pattern
    Behavior,
    /// Anomaly detected
    Anomaly,
    /// Resource usage
    Resource,
    /// Communication pattern
    Communication,
    /// Output analysis
    Output,
    /// Error pattern
    Error,
    /// Potential innovation
    Innovation,
    /// Security concern
    Security,
}

/// Disassembly report after analyzing a module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisassemblyReport {
    /// When disassembly completed
    pub completed_at: u64,
    /// Architecture summary
    pub architecture_summary: String,
    /// Identified components
    pub components: Vec<ComponentAnalysis>,
    /// Innovations discovered
    pub innovations: Vec<Innovation>,
    /// Exploits/vulnerabilities found
    pub exploits: Vec<ExploitAnalysis>,
    /// Corrupted/damaged parts
    pub corrupted_parts: Vec<String>,
    /// Useful heuristics extracted
    pub heuristics: Vec<String>,
    /// Recommended verdict
    pub recommended_verdict: ScrapyardVerdict,
    /// Overall danger level (0.0 - 1.0)
    pub danger_level: f64,
    /// Recyclability score (0.0 - 1.0)
    pub recyclability: f64,
}

/// Analysis of a component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentAnalysis {
    pub name: String,
    pub function: String,
    pub health: ComponentHealth,
    pub useful: bool,
    pub notes: String,
}

/// Component health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComponentHealth {
    Healthy,
    Degraded,
    Corrupted,
    Malicious,
    Unknown,
}

/// Innovation discovered during disassembly
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Innovation {
    pub id: String,
    pub description: String,
    pub innovation_type: InnovationType,
    pub significance: f64,
    pub extractable: bool,
    pub notes: String,
}

/// Types of innovations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InnovationType {
    /// New optimization technique
    Optimization,
    /// Novel strategy
    Strategy,
    /// Efficiency improvement
    Efficiency,
    /// Security technique
    Security,
    /// Communication method
    Communication,
    /// Learning technique
    Learning,
    /// Error recovery method
    ErrorRecovery,
}

/// Exploit analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExploitAnalysis {
    pub exploit_type: String,
    pub severity: ExploitSeverity,
    pub description: String,
    pub was_intentional: bool,
    pub can_be_weaponized: bool,
    pub mitigation: String,
}

/// Exploit severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ExploitSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Knowledge recycled back into the swarm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecycledKnowledge {
    /// Source module
    pub source_module: String,
    /// Knowledge type
    pub knowledge_type: RecyclableType,
    /// The actual knowledge
    pub content: String,
    /// Target systems for injection
    pub targets: Vec<String>,
    /// When recycled
    pub recycled_at: u64,
    /// Confidence score
    pub confidence: f64,
}

/// Execution record (destroyed modules)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub module_id: String,
    pub executed_at: u64,
    pub reason: String,
    pub blacklisted: bool,
    pub knowledge_preserved: Vec<String>,
}

/// The Scrapyard system
pub struct Scrapyard {
    /// Modules currently in quarantine/processing
    modules: HashMap<String, ScrapyardModule>,
    /// Queue for processing
    processing_queue: VecDeque<String>,
    /// Recycled knowledge bank
    knowledge_bank: Vec<RecycledKnowledge>,
    /// Execution records
    executions: Vec<ExecutionRecord>,
    /// Blacklisted module IDs
    blacklist: Vec<String>,
    /// Started at
    started_at: Instant,
    /// Total modules processed
    total_processed: u64,
    /// Total knowledge recycled
    total_recycled: u64,
}

impl Default for Scrapyard {
    fn default() -> Self {
        Self::new()
    }
}

impl Scrapyard {
    /// Create new Scrapyard
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            processing_queue: VecDeque::new(),
            knowledge_bank: Vec::new(),
            executions: Vec::new(),
            blacklist: Vec::new(),
            started_at: Instant::now(),
            total_processed: 0,
            total_recycled: 0,
        }
    }

    /// Quarantine a module
    pub fn quarantine(&mut self, module_id: String, reason: QuarantineReason) {
        // Check if blacklisted
        if self.blacklist.contains(&module_id) {
            // Immediately destroy blacklisted modules
            self.immediate_destroy(&module_id, "Blacklisted module");
            return;
        }

        // Check capacity
        if self.modules.len() >= MAX_QUARANTINE_SIZE {
            // Process oldest first
            if let Some(oldest) = self.processing_queue.pop_front() {
                self.force_verdict(&oldest);
            }
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let module = ScrapyardModule {
            module_id: module_id.clone(),
            module_type: "unknown".to_string(),
            reason,
            stage: ScrapyardStage::Quarantine,
            quarantined_at: timestamp,
            observations: Vec::new(),
            disassembly: None,
            verdict: None,
            tags: Vec::new(),
        };

        self.modules.insert(module_id.clone(), module);
        self.processing_queue.push_back(module_id);
    }

    /// Add observation to quarantined module
    pub fn add_observation(
        &mut self,
        module_id: &str,
        observer: &str,
        obs_type: ObservationType,
        content: String,
        significance: f64,
    ) {
        if let Some(module) = self.modules.get_mut(module_id) {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            module.observations.push(Observation {
                timestamp,
                observer: observer.to_string(),
                observation_type: obs_type,
                content,
                significance,
            });
        }
    }

    /// Begin disassembly of a module
    pub async fn begin_disassembly(&mut self, module_id: &str) -> Option<DisassemblyReport> {
        // Check if module exists and is in quarantine
        {
            let module = self.modules.get(module_id)?;
            if module.stage != ScrapyardStage::Quarantine {
                return None;
            }
        }

        // Update stage
        if let Some(module) = self.modules.get_mut(module_id) {
            module.stage = ScrapyardStage::Disassembly;
        }

        // Clone module for analysis (to avoid borrow issues)
        let module_clone = self.modules.get(module_id)?.clone();

        // Analyze the module
        let report = self.analyze_module(&module_clone).await;

        // Update module with report
        if let Some(module) = self.modules.get_mut(module_id) {
            module.disassembly = Some(report.clone());
            module.stage = ScrapyardStage::PendingVerdict;
        }

        Some(report)
    }

    /// Analyze a module during disassembly
    async fn analyze_module(&self, module: &ScrapyardModule) -> DisassemblyReport {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Analyze observations for patterns
        let mut innovations = Vec::new();
        let mut exploits = Vec::new();
        let mut heuristics = Vec::new();

        // Check observations for innovations
        for obs in &module.observations {
            if obs.observation_type == ObservationType::Innovation && obs.significance > 0.5 {
                innovations.push(Innovation {
                    id: format!("{}_{}", module.module_id, innovations.len()),
                    description: obs.content.clone(),
                    innovation_type: InnovationType::Optimization,
                    significance: obs.significance,
                    extractable: true,
                    notes: "Discovered during quarantine observation".to_string(),
                });
            }

            if obs.observation_type == ObservationType::Security {
                exploits.push(ExploitAnalysis {
                    exploit_type: "ObservedBehavior".to_string(),
                    severity: ExploitSeverity::Medium,
                    description: obs.content.clone(),
                    was_intentional: false,
                    can_be_weaponized: false,
                    mitigation: "Patch or isolate".to_string(),
                });
            }
        }

        // Extract heuristics from error patterns
        let error_obs: Vec<_> = module
            .observations
            .iter()
            .filter(|o| o.observation_type == ObservationType::Error)
            .collect();

        if !error_obs.is_empty() {
            heuristics.push(format!(
                "Error patterns: {} unique error types observed",
                error_obs.len()
            ));
        }

        // Calculate danger level
        let danger_level = match &module.reason {
            QuarantineReason::Dangerous { .. } => 0.9,
            QuarantineReason::Gaming { .. } => 0.7,
            QuarantineReason::Exploitable { .. } => 0.6,
            QuarantineReason::HighRisk { .. } => 0.5,
            QuarantineReason::Runaway { .. } => 0.4,
            QuarantineReason::Unstable { .. } => 0.3,
            _ => 0.2,
        };

        // Calculate recyclability
        let recyclability = if !innovations.is_empty() {
            0.7 + (innovations.len() as f64 * 0.1).min(0.3)
        } else if !heuristics.is_empty() {
            0.4
        } else {
            0.1
        };

        // Determine recommended verdict
        let recommended_verdict = if danger_level > 0.8 {
            ScrapyardVerdict::Execute {
                reason: "High danger level".to_string(),
                blacklist: true,
            }
        } else if recyclability > 0.5 {
            let useful_parts = innovations
                .iter()
                .map(|i| RecyclablePart {
                    id: i.id.clone(),
                    part_type: RecyclableType::Algorithm,
                    content: i.description.clone(),
                    confidence: i.significance,
                    inject_targets: vec!["warden".to_string(), "evolution".to_string()],
                })
                .collect();

            ScrapyardVerdict::Recycle {
                useful_parts,
                justification: format!(
                    "Found {} innovations worth preserving",
                    innovations.len()
                ),
            }
        } else {
            ScrapyardVerdict::Execute {
                reason: "Low recyclability, not worth preserving".to_string(),
                blacklist: false,
            }
        };

        DisassemblyReport {
            completed_at: timestamp,
            architecture_summary: format!(
                "Module {} analyzed: {} observations processed",
                module.module_id,
                module.observations.len()
            ),
            components: vec![ComponentAnalysis {
                name: "core".to_string(),
                function: "Main logic".to_string(),
                health: if danger_level > 0.5 {
                    ComponentHealth::Corrupted
                } else {
                    ComponentHealth::Degraded
                },
                useful: recyclability > 0.3,
                notes: "Analyzed from observations".to_string(),
            }],
            innovations,
            exploits,
            corrupted_parts: vec![],
            heuristics,
            recommended_verdict,
            danger_level,
            recyclability,
        }
    }

    /// Apply verdict to a module
    pub fn apply_verdict(&mut self, module_id: &str, verdict: ScrapyardVerdict) -> bool {
        // Check if module exists first
        if !self.modules.contains_key(module_id) {
            return false;
        }

        // Update verdict and stage first
        if let Some(module) = self.modules.get_mut(module_id) {
            module.verdict = Some(verdict.clone());
        }

        // Process verdict based on type
        match verdict {
            ScrapyardVerdict::Recycle { useful_parts, .. } => {
                // Update stage
                if let Some(module) = self.modules.get_mut(module_id) {
                    module.stage = ScrapyardStage::Recycling;
                }
                // Recycle parts
                self.recycle_parts(module_id, useful_parts);
                // Mark complete
                if let Some(module) = self.modules.get_mut(module_id) {
                    module.stage = ScrapyardStage::Complete;
                }
                self.total_processed += 1;
            }
            ScrapyardVerdict::Execute { reason, blacklist } => {
                // Update stage
                if let Some(module) = self.modules.get_mut(module_id) {
                    module.stage = ScrapyardStage::Execution;
                }
                // Execute (removes from modules)
                self.execute_module(module_id, &reason, blacklist);
                self.total_processed += 1;
            }
            ScrapyardVerdict::Rehabilitate { .. } => {
                // Return to service - remove from scrapyard
                self.modules.remove(module_id);
                self.total_processed += 1;
            }
            ScrapyardVerdict::ExtendObservation { .. } => {
                if let Some(module) = self.modules.get_mut(module_id) {
                    module.stage = ScrapyardStage::Quarantine;
                }
            }
        }

        true
    }

    /// Recycle useful parts from a module
    fn recycle_parts(&mut self, module_id: &str, parts: Vec<RecyclablePart>) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        for part in parts {
            let knowledge = RecycledKnowledge {
                source_module: module_id.to_string(),
                knowledge_type: part.part_type,
                content: part.content,
                targets: part.inject_targets,
                recycled_at: timestamp,
                confidence: part.confidence,
            };

            self.knowledge_bank.push(knowledge);
            self.total_recycled += 1;
        }
    }

    /// Execute (destroy) a module
    fn execute_module(&mut self, module_id: &str, reason: &str, blacklist: bool) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let record = ExecutionRecord {
            module_id: module_id.to_string(),
            executed_at: timestamp,
            reason: reason.to_string(),
            blacklisted: blacklist,
            knowledge_preserved: vec![],
        };

        self.executions.push(record);

        if blacklist {
            self.blacklist.push(module_id.to_string());
        }

        self.modules.remove(module_id);
    }

    /// Immediate destroy (for blacklisted)
    fn immediate_destroy(&mut self, module_id: &str, reason: &str) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.executions.push(ExecutionRecord {
            module_id: module_id.to_string(),
            executed_at: timestamp,
            reason: reason.to_string(),
            blacklisted: true,
            knowledge_preserved: vec![],
        });

        self.total_processed += 1;
    }

    /// Force a verdict on a module (for capacity management)
    fn force_verdict(&mut self, module_id: &str) {
        if let Some(module) = self.modules.get(module_id) {
            // Use disassembly recommendation if available
            let verdict = if let Some(ref disassembly) = module.disassembly {
                disassembly.recommended_verdict.clone()
            } else {
                // Default to execution
                ScrapyardVerdict::Execute {
                    reason: "Forced due to capacity".to_string(),
                    blacklist: false,
                }
            };

            self.apply_verdict(module_id, verdict);
        }
    }

    /// Process all modules ready for processing
    pub async fn process_all(&mut self) -> Vec<RecycledKnowledge> {
        let ready_ids: Vec<_> = self
            .modules
            .iter()
            .filter(|(_, m)| m.stage == ScrapyardStage::Quarantine)
            .filter(|(_, m)| {
                let elapsed = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    - m.quarantined_at;
                elapsed > MIN_OBSERVATION_TIME.as_secs()
            })
            .map(|(id, _)| id.clone())
            .collect();

        for module_id in ready_ids {
            // Disassemble
            if let Some(report) = self.begin_disassembly(&module_id).await {
                // Apply recommended verdict
                self.apply_verdict(&module_id, report.recommended_verdict);
            }
        }

        // Return newly recycled knowledge
        self.knowledge_bank.clone()
    }

    /// Get all recycled knowledge
    pub fn knowledge_bank(&self) -> &[RecycledKnowledge] {
        &self.knowledge_bank
    }

    /// Get knowledge for a specific target
    pub fn knowledge_for_target(&self, target: &str) -> Vec<&RecycledKnowledge> {
        self.knowledge_bank
            .iter()
            .filter(|k| k.targets.iter().any(|t| t == target))
            .collect()
    }

    /// Is a module blacklisted?
    pub fn is_blacklisted(&self, module_id: &str) -> bool {
        self.blacklist.contains(&module_id.to_string())
    }

    /// Get quarantine count
    pub fn quarantine_count(&self) -> usize {
        self.modules.len()
    }

    /// Get stats
    pub fn stats(&self) -> ScrapyardStats {
        ScrapyardStats {
            quarantined: self.modules.len(),
            processed: self.total_processed,
            recycled: self.total_recycled,
            executed: self.executions.len() as u64,
            blacklisted: self.blacklist.len(),
            knowledge_items: self.knowledge_bank.len(),
            uptime: self.started_at.elapsed(),
        }
    }
}

/// Scrapyard statistics
#[derive(Debug, Clone)]
pub struct ScrapyardStats {
    pub quarantined: usize,
    pub processed: u64,
    pub recycled: u64,
    pub executed: u64,
    pub blacklisted: usize,
    pub knowledge_items: usize,
    pub uptime: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scrapyard_creation() {
        let scrapyard = Scrapyard::new();
        assert_eq!(scrapyard.quarantine_count(), 0);
    }

    #[test]
    fn test_quarantine() {
        let mut scrapyard = Scrapyard::new();

        scrapyard.quarantine(
            "test_module".to_string(),
            QuarantineReason::Erratic {
                description: "Acting weird".to_string(),
            },
        );

        assert_eq!(scrapyard.quarantine_count(), 1);
        assert!(!scrapyard.is_blacklisted("test_module"));
    }

    #[test]
    fn test_add_observation() {
        let mut scrapyard = Scrapyard::new();

        scrapyard.quarantine(
            "test_module".to_string(),
            QuarantineReason::Unprofitable {
                loss_amount: 100.0,
                period: "1h".to_string(),
            },
        );

        scrapyard.add_observation(
            "test_module",
            "auditor",
            ObservationType::Behavior,
            "Interesting pattern observed".to_string(),
            0.7,
        );

        let module = scrapyard.modules.get("test_module").unwrap();
        assert_eq!(module.observations.len(), 1);
    }

    #[test]
    fn test_verdict_execute() {
        let mut scrapyard = Scrapyard::new();

        scrapyard.quarantine(
            "dangerous_module".to_string(),
            QuarantineReason::Dangerous {
                threat_type: "Malicious".to_string(),
            },
        );

        let success = scrapyard.apply_verdict(
            "dangerous_module",
            ScrapyardVerdict::Execute {
                reason: "Too dangerous".to_string(),
                blacklist: true,
            },
        );

        assert!(success);
        assert!(scrapyard.is_blacklisted("dangerous_module"));
        assert_eq!(scrapyard.quarantine_count(), 0);
    }

    #[test]
    fn test_verdict_recycle() {
        let mut scrapyard = Scrapyard::new();

        scrapyard.quarantine(
            "useful_module".to_string(),
            QuarantineReason::Inefficient {
                efficiency: 0.3,
                threshold: 0.5,
            },
        );

        let success = scrapyard.apply_verdict(
            "useful_module",
            ScrapyardVerdict::Recycle {
                useful_parts: vec![RecyclablePart {
                    id: "algo_1".to_string(),
                    part_type: RecyclableType::Algorithm,
                    content: "Useful algorithm".to_string(),
                    confidence: 0.8,
                    inject_targets: vec!["warden".to_string()],
                }],
                justification: "Has useful parts".to_string(),
            },
        );

        assert!(success);
        assert_eq!(scrapyard.knowledge_bank().len(), 1);
        assert!(!scrapyard.is_blacklisted("useful_module"));
    }

    #[test]
    fn test_blacklist_immediate_destroy() {
        let mut scrapyard = Scrapyard::new();

        // First, blacklist a module
        scrapyard.quarantine(
            "bad_module".to_string(),
            QuarantineReason::Gaming {
                strategy: "Fake results".to_string(),
            },
        );
        scrapyard.apply_verdict(
            "bad_module",
            ScrapyardVerdict::Execute {
                reason: "Gaming".to_string(),
                blacklist: true,
            },
        );

        // Now try to quarantine again
        scrapyard.quarantine(
            "bad_module".to_string(),
            QuarantineReason::Erratic {
                description: "Back again".to_string(),
            },
        );

        // Should be immediately destroyed
        assert_eq!(scrapyard.quarantine_count(), 0);
        assert!(scrapyard.is_blacklisted("bad_module"));
    }

    #[test]
    fn test_knowledge_for_target() {
        let mut scrapyard = Scrapyard::new();

        scrapyard.quarantine(
            "module_1".to_string(),
            QuarantineReason::Inefficient {
                efficiency: 0.3,
                threshold: 0.5,
            },
        );

        scrapyard.apply_verdict(
            "module_1",
            ScrapyardVerdict::Recycle {
                useful_parts: vec![RecyclablePart {
                    id: "algo_1".to_string(),
                    part_type: RecyclableType::Algorithm,
                    content: "For warden".to_string(),
                    confidence: 0.8,
                    inject_targets: vec!["warden".to_string()],
                }],
                justification: "Has warden knowledge".to_string(),
            },
        );

        let warden_knowledge = scrapyard.knowledge_for_target("warden");
        assert_eq!(warden_knowledge.len(), 1);

        let evolution_knowledge = scrapyard.knowledge_for_target("evolution");
        assert!(evolution_knowledge.is_empty());
    }

    #[test]
    fn test_stats() {
        let mut scrapyard = Scrapyard::new();

        scrapyard.quarantine(
            "module_1".to_string(),
            QuarantineReason::Erratic {
                description: "Test".to_string(),
            },
        );

        let stats = scrapyard.stats();
        assert_eq!(stats.quarantined, 1);
        assert_eq!(stats.processed, 0);
    }
}
