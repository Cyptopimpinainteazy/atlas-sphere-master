//! X3 Swarm Intelligence - Cultural Organism Module
//!
//! This module extends the quantum-swarm with cultural/marketing intelligence
//! capabilities. It implements the distributed cultural organism architecture.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// AGENT TYPES
// ============================================================================

/// Agent categories in the swarm
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentCategory {
    /// Discover people, platforms, protocols
    Scout,
    /// Learn tone, values, taboos
    Linguist,
    /// Initiate contact (lowest volume, highest quality)
    Envoy,
    /// Quietly reinforce what others publish
    Amplifier,
    /// Track responses, warmth, ignores
    Archivist,
    /// Find friction in the chain
    Introspector,
    /// Propose protocol changes
    Mutator,
    /// Create apps and tooling
    Builder,
    /// Shape narrative
    Sculptor,
    /// Check language consistency
    Auditor,
    /// Counter criticism
    Defender,
    /// Create curiosity gaps
    Converter,
}

/// Agent lifecycle state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentState {
    /// Being initialized
    Spawning {
        mandate: Mandate,
        eval_window_ms: u64,
    },
    /// Actively operating
    Active { last_improvement: u64 },
    /// Performance warning issued
    Warning {
        reason: String,
        grace_period_ms: u64,
        expires_at: u64,
    },
    /// Scheduled for termination
    ScheduledForDeath {
        reason: DeathReason,
        execute_at: u64,
    },
    /// Terminated
    Terminated {
        reason: DeathReason,
        post_mortem: Option<PostMortem>,
    },
}

/// Reasons an agent can be terminated
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeathReason {
    FailedToImproveMetric,
    StabilizedIntoBehavior,
    BecamePredictable,
    RequiredJustificationToExist,
    OverlappedTooLong,
    OptimizedWrongMetric,
    DetectedSpamBehavior,
    ExceededAuthority,
    ConstitutionViolation,
}

/// Agent mandate - narrow scope required
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mandate {
    pub scope: String,
    pub objectives: Vec<String>,
    pub constraints: Vec<String>,
    pub evaluation_window_ms: u64,
    pub death_conditions: Vec<DeathCondition>,
}

/// Condition that triggers agent death
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeathCondition {
    pub metric: String,
    pub operator: ComparisonOperator,
    pub threshold: f64,
    pub window_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComparisonOperator {
    LessThan,
    GreaterThan,
    Equals,
    Plateau,
}

/// Post-mortem data for terminated agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostMortem {
    pub agent_id: String,
    pub category: AgentCategory,
    pub lifespan_ms: u64,
    pub final_metrics: HashMap<String, f64>,
    pub reason: DeathReason,
    pub lessons: Vec<String>,
    pub memory_preserved: bool,
}

/// A swarm agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CulturalAgent {
    pub id: String,
    pub category: AgentCategory,
    pub state: AgentState,
    pub mandate: Mandate,
    pub created_at: u64,
    pub metrics: HashMap<String, MetricSeries>,
}

/// Time series of metric values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricSeries {
    pub values: Vec<f64>,
    pub timestamps: Vec<u64>,
    pub trend: Trend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Trend {
    Improving,
    Stable,
    Declining,
}

impl CulturalAgent {
    /// Create a new agent
    pub fn new(id: String, category: AgentCategory, mandate: Mandate) -> Self {
        Self {
            id,
            category,
            state: AgentState::Spawning {
                mandate: mandate.clone(),
                eval_window_ms: mandate.evaluation_window_ms,
            },
            mandate,
            created_at: now_ms(),
            metrics: HashMap::new(),
        }
    }

    /// Record a metric value
    pub fn record_metric(&mut self, name: &str, value: f64) {
        let ts = now_ms();
        let series = self
            .metrics
            .entry(name.to_string())
            .or_insert_with(|| MetricSeries {
                values: Vec::new(),
                timestamps: Vec::new(),
                trend: Trend::Stable,
            });
        series.values.push(value);
        series.timestamps.push(ts);
        series.update_trend();
    }

    /// Schedule death
    pub fn schedule_death(&mut self, reason: DeathReason, delay_ms: u64) {
        self.state = AgentState::ScheduledForDeath {
            reason,
            execute_at: now_ms() + delay_ms,
        };
    }

    /// Check if agent should die based on conditions
    pub fn evaluate_death_conditions(&self) -> Option<DeathReason> {
        for condition in &self.mandate.death_conditions {
            if let Some(series) = self.metrics.get(&condition.metric) {
                if series.check_condition(condition) {
                    return Some(DeathReason::FailedToImproveMetric);
                }
            }
        }

        // Check for behavioral stagnation
        let all_stable = self.metrics.values().all(|s| s.trend == Trend::Stable);
        if all_stable && self.age_ms() > self.mandate.evaluation_window_ms * 3 {
            return Some(DeathReason::StabilizedIntoBehavior);
        }

        None
    }

    fn age_ms(&self) -> u64 {
        now_ms().saturating_sub(self.created_at)
    }
}

impl MetricSeries {
    fn update_trend(&mut self) {
        if self.values.len() < 3 {
            self.trend = Trend::Stable;
            return;
        }

        let recent: Vec<f64> = self.values.iter().rev().take(10).copied().collect();
        if recent.len() < 3 {
            self.trend = Trend::Stable;
            return;
        }

        let first_half_avg: f64 =
            recent[recent.len() / 2..].iter().sum::<f64>() / (recent.len() / 2) as f64;
        let second_half_avg: f64 =
            recent[..recent.len() / 2].iter().sum::<f64>() / (recent.len() / 2) as f64;

        let delta = second_half_avg - first_half_avg;
        let threshold = first_half_avg.abs() * 0.05; // 5% change threshold

        self.trend = if delta > threshold {
            Trend::Improving
        } else if delta < -threshold {
            Trend::Declining
        } else {
            Trend::Stable
        };
    }

    fn check_condition(&self, condition: &DeathCondition) -> bool {
        if self.values.is_empty() {
            return false;
        }

        let recent_avg = if self.values.len() >= 10 {
            self.values.iter().rev().take(10).sum::<f64>() / 10.0
        } else {
            self.values.iter().sum::<f64>() / self.values.len() as f64
        };

        match condition.operator {
            ComparisonOperator::LessThan => recent_avg < condition.threshold,
            ComparisonOperator::GreaterThan => recent_avg > condition.threshold,
            ComparisonOperator::Equals => (recent_avg - condition.threshold).abs() < 0.001,
            ComparisonOperator::Plateau => self.trend == Trend::Stable,
        }
    }
}

// ============================================================================
// SYNTHETIC IDENTITY TYPES
// ============================================================================

/// Archetype for synthetic personas
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PersonaArchetype {
    CryptoEarlyBitterHopeful,
    NumbersGuy,
    CynicalBuilder,
    ExhaustedFounder,
    TraderDoesntShill,
    ProtocolResearcher,
    DefiDegenReformed,
    InstitutionalCurious,
}

/// Platform-specific tone configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformTone {
    pub twitter: TwitterTone,
    pub linkedin: LinkedInTone,
    pub reddit: RedditTone,
    pub medium: MediumTone,
    pub discord: DiscordTone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TwitterTone {
    Snark,
    Shitpost,
    ThreadEnergy,
    Minimalist,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkedInTone {
    VirtueSignal,
    ThoughtLeader,
    HumbleBrag,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RedditTone {
    Pedantic,
    Helpful,
    Contrarian,
    Lurker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediumTone {
    LongformExplainer,
    PersonalJourney,
    TechnicalDeepDive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiscordTone {
    Casual,
    Supportive,
    Sarcastic,
}

/// Ideological drift over time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeologicalDrift {
    /// Initial positions: topic -> stance (-1.0 to 1.0)
    pub initial_positions: HashMap<String, f64>,
    /// How fast opinions shift
    pub drift_rate: f64,
    /// Tolerance for contradictions (0-1)
    pub contradiction_tolerance: f64,
    /// History of position changes
    pub drift_history: Vec<DriftEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftEvent {
    pub topic: String,
    pub old_stance: f64,
    pub new_stance: f64,
    pub timestamp: u64,
}

/// Posting behavior patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostingBehavior {
    pub fatigue_cycle: FatigueCycle,
    pub obsession_cycles: Vec<ObsessionCycle>,
    pub platform_activity: HashMap<String, PlatformActivity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FatigueCycle {
    pub active_phase_ms: u64,
    pub rest_phase_ms: u64,
    pub current_phase: FatiguePhase,
    pub phase_started_at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FatiguePhase {
    Active,
    Resting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObsessionCycle {
    pub topic: String,
    pub intensity: f64, // 0-1
    pub started_at: u64,
    pub expected_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformActivity {
    pub posts_per_day: f64,
    pub peak_hours: Vec<u8>,
    pub response_latency_ms: u64,
}

/// A synthetic identity for presence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntheticIdentity {
    pub id: String,
    pub archetype: PersonaArchetype,

    // Narrative
    pub backstory: Backstory,

    // Behavior
    pub ideological_drift: IdeologicalDrift,
    pub posting_behavior: PostingBehavior,
    pub platform_tone: PlatformTone,

    // Key characteristics (all must be true)
    pub slightly_wrong_sometimes: bool,
    pub emotionally_consistent: bool,
    pub logically_imperfect: bool,
    pub opinionated_not_maximalist: bool,
    pub visibly_learning: bool,

    // Platform accounts
    pub accounts: HashMap<String, PlatformAccount>,

    // Trust accumulation (grows slowly)
    pub trust_score: f64,
    pub trust_history: Vec<TrustEvent>,

    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backstory {
    pub summary: String,
    pub key_events: Vec<String>,
    pub beliefs: Vec<String>,
    pub contradictions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformAccount {
    pub platform: String,
    pub handle: String,
    pub display_name: String,
    pub bio: String,
    pub created_at: u64,
    pub follower_count: u64,
    pub following_count: u64,
    pub post_count: u64,
    pub engagement_rate: f64,
    pub last_active: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustEvent {
    pub event: String,
    pub delta: f64,
    pub timestamp: u64,
}

// ============================================================================
// SWARM CONSTITUTION
// ============================================================================

/// The constitution that governs swarm behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmConstitution {
    /// Rules the swarm lives by
    pub lives_by: Vec<String>,
    /// Rules that cause swarm death
    pub dies_by: Vec<String>,
    /// Agent lifecycle rules
    pub agent_rules: AgentRules,
    /// Content generation rules
    pub content_rules: ContentRules,
    /// Outreach rules
    pub outreach_rules: OutreachRules,
    /// Defense rules
    pub defense_rules: DefenseRules,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRules {
    pub max_lifespan_ms: Option<u64>,
    pub evaluation_interval_ms: u64,
    pub overlap_tolerance_ms: u64,
    pub sentiment_is_not_metric: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentRules {
    pub never_repeat: bool,
    pub mutate_always: bool,
    pub no_ctas: bool,
    pub no_urgency: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutreachRules {
    pub one_to_one_only: bool,
    pub no_blast: bool,
    pub require_prior_context: bool,
    pub never_beg: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefenseRules {
    pub never_dogpile: bool,
    pub never_deny_obvious: bool,
    pub never_attack_person: bool,
    pub never_show_emotion: bool,
}

impl Default for SwarmConstitution {
    fn default() -> Self {
        Self {
            lives_by: vec![
                "Never lie about measurable reality".to_string(),
                "Never spam".to_string(),
                "Never beg".to_string(),
                "Never defend sunk cost".to_string(),
                "Never confuse attention with trust".to_string(),
                "Credibility first, allure later".to_string(),
                "Competence ages well, sex appeal decays fast".to_string(),
            ],
            dies_by: vec![
                "Stagnation".to_string(),
                "Repetition".to_string(),
                "Internal politicking".to_string(),
                "Protecting agents over outcomes".to_string(),
                "Optimizing clicks at expense of long-term trust".to_string(),
                "Using attraction to compensate for missing substance".to_string(),
            ],
            agent_rules: AgentRules {
                max_lifespan_ms: None,
                evaluation_interval_ms: 3600_000, // 1 hour
                overlap_tolerance_ms: 86400_000,  // 24 hours
                sentiment_is_not_metric: true,
            },
            content_rules: ContentRules {
                never_repeat: true,
                mutate_always: true,
                no_ctas: true,
                no_urgency: true,
            },
            outreach_rules: OutreachRules {
                one_to_one_only: true,
                no_blast: true,
                require_prior_context: true,
                never_beg: true,
            },
            defense_rules: DefenseRules {
                never_dogpile: true,
                never_deny_obvious: true,
                never_attack_person: true,
                never_show_emotion: true,
            },
        }
    }
}

// ============================================================================
// NETWORK PHASE TYPES
// ============================================================================

/// Network phase state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkPhase {
    TestnetTruth,
    TestnetStabilizing,
    PreMainnetPositioning,
    MainnetConfidence,
}

/// Signals for phase transition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionSignals {
    pub mechanics_stable: bool,
    pub real_usage_repeat: bool,
    pub swarm_suggests_removals: bool,
}

impl TransitionSignals {
    pub fn ready_for_next_phase(&self, current: NetworkPhase) -> bool {
        match current {
            NetworkPhase::TestnetTruth => self.mechanics_stable,
            NetworkPhase::TestnetStabilizing => self.mechanics_stable && self.real_usage_repeat,
            NetworkPhase::PreMainnetPositioning => {
                self.mechanics_stable && self.real_usage_repeat && self.swarm_suggests_removals
            }
            NetworkPhase::MainnetConfidence => false, // Final phase
        }
    }
}

// ============================================================================
// CREDIBILITY CIRCLES
// ============================================================================

/// Credibility circle for influence staging
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CredibilityCircle {
    /// Core: Nerdy, battle-tested (builders, traders, devs, analysts)
    Core,
    /// Secondary: Aspirational, stylish (media, presenters, ambassadors)
    Secondary,
    /// Tertiary: Mass market, non-tech (tutorials, stories, approachable UI)
    Tertiary,
}

impl CredibilityCircle {
    pub fn activation_order(&self) -> u8 {
        match self {
            Self::Core => 1,
            Self::Secondary => 2,
            Self::Tertiary => 3,
        }
    }

    pub fn prerequisites(&self) -> Vec<&'static str> {
        match self {
            Self::Core => vec![],
            Self::Secondary => vec!["core_stable", "messaging_works"],
            Self::Tertiary => vec!["credibility_established", "aspirational_visuals_ready"],
        }
    }
}

// ============================================================================
// MODEL TRAINING PHASE
// ============================================================================

/// Model training phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelPhase {
    /// Use off-the-shelf models, build judgment
    BorrowBrains,
    /// Fine-tune narrow specialists where failures persist
    FineTune,
    /// Distill institutional memory into core model
    Distill,
    /// Full sovereignty if justified
    Sovereignty,
}

impl ModelPhase {
    pub fn description(&self) -> &'static str {
        match self {
            Self::BorrowBrains => "Train selection pressure, not intelligence",
            Self::FineTune => "Taste encoded in weights",
            Self::Distill => "House style brain",
            Self::Sovereignty => "Formalizing the guild",
        }
    }
}

// ============================================================================
// HELPERS
// ============================================================================

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constitution_default() {
        let constitution = SwarmConstitution::default();
        assert!(!constitution.lives_by.is_empty());
        assert!(!constitution.dies_by.is_empty());
        assert!(constitution.agent_rules.sentiment_is_not_metric);
    }

    #[test]
    fn test_agent_creation() {
        let mandate = Mandate {
            scope: "scout".to_string(),
            objectives: vec!["discover targets".to_string()],
            constraints: vec!["no spam".to_string()],
            evaluation_window_ms: 3600_000,
            death_conditions: vec![],
        };

        let agent = CulturalAgent::new("test-001".to_string(), AgentCategory::Scout, mandate);
        assert_eq!(agent.category, AgentCategory::Scout);
        assert!(matches!(agent.state, AgentState::Spawning { .. }));
    }

    #[test]
    fn test_transition_signals() {
        let signals = TransitionSignals {
            mechanics_stable: true,
            real_usage_repeat: true,
            swarm_suggests_removals: false,
        };

        assert!(signals.ready_for_next_phase(NetworkPhase::TestnetTruth));
        assert!(signals.ready_for_next_phase(NetworkPhase::TestnetStabilizing));
        assert!(!signals.ready_for_next_phase(NetworkPhase::PreMainnetPositioning));
    }
}
