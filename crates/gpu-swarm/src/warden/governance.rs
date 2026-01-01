//! Governance and Emergency Override
//!
//! Handles emergency situations, guard integration, and governance actions.

use crate::warden::policy::ComputeLane;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Threat level classifications
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ThreatLevel {
    /// No active threats
    None,
    /// Minor anomalies detected
    Low,
    /// Suspicious activity, increased monitoring
    Elevated,
    /// Active threat, defensive measures engaged
    High,
    /// Critical threat, emergency protocols active
    Critical,
}

impl Default for ThreatLevel {
    fn default() -> Self {
        Self::None
    }
}

impl ThreatLevel {
    /// Get security resource multiplier for this threat level
    pub fn security_multiplier(&self) -> f64 {
        match self {
            Self::None => 1.0,
            Self::Low => 1.2,
            Self::Elevated => 1.5,
            Self::High => 2.0,
            Self::Critical => 4.0,
        }
    }

    /// Check if this level requires emergency response
    pub fn is_emergency(&self) -> bool {
        matches!(self, Self::High | Self::Critical)
    }

    /// Get recommended action
    pub fn recommended_action(&self) -> &'static str {
        match self {
            Self::None => "Normal operations",
            Self::Low => "Increased monitoring",
            Self::Elevated => "Activate secondary guards",
            Self::High => "Emergency reallocation",
            Self::Critical => "Full defensive mode",
        }
    }
}

/// Types of governance actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GovernanceAction {
    /// Emergency security boost
    EmergencySecurityBoost { target_percent: f64, reason: String },
    /// Halt a specific lane
    HaltLane { lane: ComputeLane, reason: String },
    /// Resume a halted lane
    ResumeLane { lane: ComputeLane },
    /// Force reallocation
    ForceReallocation {
        allocations: HashMap<ComputeLane, f64>,
        reason: String,
    },
    /// Enable/disable evolution
    SetEvolutionEnabled { enabled: bool },
    /// Update threat level
    UpdateThreatLevel { level: ThreatLevel, source: String },
    /// Blacklist a node
    BlacklistNode {
        node_id: String,
        reason: String,
        duration: Option<Duration>,
    },
    /// Whitelist a node
    WhitelistNode { node_id: String },
    /// Pause all non-critical operations
    PauseNonCritical { duration: Duration },
    /// Resume normal operations
    ResumeNormal,
}

impl GovernanceAction {
    /// Get priority (higher = more urgent)
    pub fn priority(&self) -> u8 {
        match self {
            Self::EmergencySecurityBoost { .. } => 100,
            Self::HaltLane { .. } => 90,
            Self::BlacklistNode { .. } => 85,
            Self::PauseNonCritical { .. } => 80,
            Self::UpdateThreatLevel { .. } => 75,
            Self::ForceReallocation { .. } => 60,
            Self::ResumeLane { .. } => 40,
            Self::WhitelistNode { .. } => 30,
            Self::ResumeNormal => 20,
            Self::SetEvolutionEnabled { .. } => 10,
        }
    }

    /// Check if this action is reversible
    pub fn is_reversible(&self) -> bool {
        !matches!(self, Self::BlacklistNode { .. })
    }
}

/// An emergency override request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyOverride {
    /// Unique override ID
    pub id: String,
    /// Action to take
    pub action: GovernanceAction,
    /// Who initiated (guard ID, admin, or system)
    pub initiated_by: String,
    /// Reason for override
    pub reason: String,
    /// When override was created
    pub created_at_ms: u64,
    /// Override expires after this duration (None = permanent until revoked)
    pub expires_after: Option<Duration>,
    /// Whether override requires multi-sig
    pub requires_multisig: bool,
    /// Approvals received (for multi-sig)
    pub approvals: Vec<String>,
    /// Required approvals (for multi-sig)
    pub required_approvals: u32,
}

impl EmergencyOverride {
    /// Create new override
    pub fn new(action: GovernanceAction, initiated_by: String, reason: String) -> Self {
        let requires_multisig = action.priority() >= 80;
        let required_approvals = if requires_multisig { 2 } else { 1 };

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            action,
            initiated_by: initiated_by.clone(),
            reason,
            created_at_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            expires_after: Some(Duration::from_secs(3600)), // 1 hour default
            requires_multisig,
            approvals: vec![initiated_by],
            required_approvals,
        }
    }

    /// Check if override is approved
    pub fn is_approved(&self) -> bool {
        self.approvals.len() as u32 >= self.required_approvals
    }

    /// Add approval
    pub fn approve(&mut self, approver: String) -> bool {
        if !self.approvals.contains(&approver) {
            self.approvals.push(approver);
            true
        } else {
            false
        }
    }

    /// Check if override has expired
    pub fn is_expired(&self, created_instant: Instant) -> bool {
        if let Some(expires) = self.expires_after {
            Instant::now().duration_since(created_instant) > expires
        } else {
            false
        }
    }
}

/// Guard bot registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardBot {
    /// Unique guard ID
    pub id: String,
    /// Guard type/specialization
    pub guard_type: GuardType,
    /// Node ID where guard runs
    pub node_id: String,
    /// Is guard active
    pub active: bool,
    /// Last heartbeat timestamp
    pub last_heartbeat_ms: u64,
    /// Threats detected
    pub threats_detected: u32,
    /// False positives
    pub false_positives: u32,
    /// Trust score (0.0 - 1.0)
    pub trust_score: f64,
}

impl GuardBot {
    pub fn new(id: String, guard_type: GuardType, node_id: String) -> Self {
        Self {
            id,
            guard_type,
            node_id,
            active: true,
            last_heartbeat_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            threats_detected: 0,
            false_positives: 0,
            trust_score: 0.8, // Start with decent trust
        }
    }

    /// Update trust score based on performance
    pub fn update_trust(&mut self, correct_detection: bool) {
        if correct_detection {
            self.threats_detected += 1;
            self.trust_score = (self.trust_score + 0.05).min(1.0);
        } else {
            self.false_positives += 1;
            self.trust_score = (self.trust_score - 0.1).max(0.0);
        }
    }

    /// Check if guard is healthy (recent heartbeat)
    pub fn is_healthy(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        // Consider unhealthy if no heartbeat in 60 seconds
        now - self.last_heartbeat_ms < 60_000
    }
}

/// Types of guard bots
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GuardType {
    /// Network traffic analysis
    NetworkMonitor,
    /// Transaction validation
    TxValidator,
    /// MEV protection
    MevGuard,
    /// Anomaly detection
    AnomalyDetector,
    /// Resource abuse detection
    ResourceGuard,
    /// Smart contract security
    ContractAuditor,
}

impl GuardType {
    /// All guard types
    pub fn all() -> Vec<Self> {
        vec![
            Self::NetworkMonitor,
            Self::TxValidator,
            Self::MevGuard,
            Self::AnomalyDetector,
            Self::ResourceGuard,
            Self::ContractAuditor,
        ]
    }

    /// Minimum guards of this type required
    pub fn min_required(&self) -> u32 {
        match self {
            Self::NetworkMonitor => 2,
            Self::TxValidator => 3,
            Self::MevGuard => 1,
            Self::AnomalyDetector => 2,
            Self::ResourceGuard => 1,
            Self::ContractAuditor => 1,
        }
    }
}

/// Governance engine manages emergency overrides and guards
pub struct GovernanceEngine {
    /// Current threat level
    threat_level: ThreatLevel,
    /// Active emergency overrides
    active_overrides: Vec<EmergencyOverride>,
    /// Override creation times (for expiry tracking)
    override_times: HashMap<String, Instant>,
    /// Registered guards
    guards: HashMap<String, GuardBot>,
    /// Halted lanes
    halted_lanes: Vec<ComputeLane>,
    /// Blacklisted nodes
    blacklist: HashMap<String, (String, Option<Instant>)>, // node_id -> (reason, expiry)
    /// Admin keys (for override approval)
    admin_keys: Vec<String>,
}

impl Default for GovernanceEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl GovernanceEngine {
    /// Create new governance engine
    pub fn new() -> Self {
        Self {
            threat_level: ThreatLevel::None,
            active_overrides: Vec::new(),
            override_times: HashMap::new(),
            guards: HashMap::new(),
            halted_lanes: Vec::new(),
            blacklist: HashMap::new(),
            admin_keys: Vec::new(),
        }
    }

    /// Add admin key
    pub fn add_admin(&mut self, key: String) {
        if !self.admin_keys.contains(&key) {
            self.admin_keys.push(key);
        }
    }

    /// Submit an emergency override
    pub fn submit_override(
        &mut self,
        mut override_req: EmergencyOverride,
    ) -> Result<String, String> {
        // Validate initiator
        let is_admin = self.admin_keys.contains(&override_req.initiated_by);
        let is_guard = self.guards.contains_key(&override_req.initiated_by);

        if !is_admin && !is_guard {
            return Err("Unauthorized: must be admin or registered guard".to_string());
        }

        // Guards can only submit certain actions
        if !is_admin {
            match &override_req.action {
                GovernanceAction::UpdateThreatLevel { .. }
                | GovernanceAction::BlacklistNode { .. } => {
                    // Guards can submit these
                }
                _ => {
                    return Err(
                        "Guards can only submit threat updates and blacklist requests".to_string(),
                    );
                }
            }
        }

        let id = override_req.id.clone();
        self.override_times.insert(id.clone(), Instant::now());
        self.active_overrides.push(override_req);

        Ok(id)
    }

    /// Approve an override (for multi-sig)
    pub fn approve_override(&mut self, override_id: &str, approver: &str) -> Result<bool, String> {
        if !self.admin_keys.contains(&approver.to_string()) {
            return Err("Unauthorized: must be admin".to_string());
        }

        for override_req in &mut self.active_overrides {
            if override_req.id == override_id {
                override_req.approve(approver.to_string());
                return Ok(override_req.is_approved());
            }
        }

        Err("Override not found".to_string())
    }

    /// Execute approved overrides
    pub fn execute_overrides(&mut self) -> Vec<GovernanceAction> {
        let mut executed = Vec::new();

        // Remove expired overrides
        let now = Instant::now();
        self.active_overrides.retain(|o| {
            if let Some(created) = self.override_times.get(&o.id) {
                !o.is_expired(*created)
            } else {
                true
            }
        });

        // Execute approved overrides
        let approved: Vec<_> = self
            .active_overrides
            .iter()
            .filter(|o| o.is_approved())
            .cloned()
            .collect();

        for override_req in approved {
            if let Some(action) = self.apply_action(&override_req.action) {
                executed.push(action);
            }

            // Remove executed override
            self.active_overrides.retain(|o| o.id != override_req.id);
            self.override_times.remove(&override_req.id);
        }

        executed
    }

    /// Apply a governance action
    fn apply_action(&mut self, action: &GovernanceAction) -> Option<GovernanceAction> {
        match action {
            GovernanceAction::UpdateThreatLevel { level, .. } => {
                self.threat_level = *level;
                Some(action.clone())
            }
            GovernanceAction::HaltLane { lane, .. } => {
                if !self.halted_lanes.contains(lane) {
                    self.halted_lanes.push(*lane);
                }
                Some(action.clone())
            }
            GovernanceAction::ResumeLane { lane } => {
                self.halted_lanes.retain(|l| l != lane);
                Some(action.clone())
            }
            GovernanceAction::BlacklistNode {
                node_id,
                reason,
                duration,
            } => {
                let expiry = duration.map(|d| Instant::now() + d);
                self.blacklist
                    .insert(node_id.clone(), (reason.clone(), expiry));
                Some(action.clone())
            }
            GovernanceAction::WhitelistNode { node_id } => {
                self.blacklist.remove(node_id);
                Some(action.clone())
            }
            _ => Some(action.clone()),
        }
    }

    /// Register a guard bot
    pub fn register_guard(&mut self, guard: GuardBot) {
        self.guards.insert(guard.id.clone(), guard);
    }

    /// Update guard heartbeat
    pub fn guard_heartbeat(&mut self, guard_id: &str) -> bool {
        if let Some(guard) = self.guards.get_mut(guard_id) {
            guard.last_heartbeat_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            true
        } else {
            false
        }
    }

    /// Get current threat level
    pub fn threat_level(&self) -> ThreatLevel {
        self.threat_level
    }

    /// Check if a lane is halted
    pub fn is_lane_halted(&self, lane: ComputeLane) -> bool {
        self.halted_lanes.contains(&lane)
    }

    /// Check if a node is blacklisted
    pub fn is_blacklisted(&self, node_id: &str) -> bool {
        if let Some((_, expiry)) = self.blacklist.get(node_id) {
            if let Some(exp) = expiry {
                if Instant::now() > *exp {
                    return false; // Expired
                }
            }
            true
        } else {
            false
        }
    }

    /// Get healthy guards count by type
    pub fn healthy_guards(&self) -> HashMap<GuardType, u32> {
        let mut counts = HashMap::new();

        for guard in self.guards.values() {
            if guard.is_healthy() && guard.active {
                *counts.entry(guard.guard_type).or_insert(0) += 1;
            }
        }

        counts
    }

    /// Check if security coverage is adequate
    pub fn has_adequate_coverage(&self) -> bool {
        let healthy = self.healthy_guards();

        for guard_type in GuardType::all() {
            let count = healthy.get(&guard_type).copied().unwrap_or(0);
            if count < guard_type.min_required() {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threat_level() {
        assert!(ThreatLevel::Critical > ThreatLevel::None);
        assert!(ThreatLevel::High.is_emergency());
        assert!(!ThreatLevel::Low.is_emergency());
    }

    #[test]
    fn test_governance_engine() {
        let mut engine = GovernanceEngine::new();
        engine.add_admin("admin1".to_string());

        // Submit override
        let override_req = EmergencyOverride::new(
            GovernanceAction::UpdateThreatLevel {
                level: ThreatLevel::Elevated,
                source: "test".to_string(),
            },
            "admin1".to_string(),
            "Test threat".to_string(),
        );

        let result = engine.submit_override(override_req);
        assert!(result.is_ok());

        // Execute
        let executed = engine.execute_overrides();
        assert!(!executed.is_empty());
        assert_eq!(engine.threat_level(), ThreatLevel::Elevated);
    }

    #[test]
    fn test_guard_registration() {
        let mut engine = GovernanceEngine::new();

        let guard = GuardBot::new(
            "guard1".to_string(),
            GuardType::NetworkMonitor,
            "node1".to_string(),
        );

        engine.register_guard(guard);
        assert!(engine.guard_heartbeat("guard1"));
    }

    #[test]
    fn test_blacklist() {
        let mut engine = GovernanceEngine::new();
        engine.add_admin("admin".to_string());
        engine.add_admin("admin2".to_string()); // Need 2 admins for high-priority actions

        // Blacklist a node (requires multi-sig for high priority)
        let mut override_req = EmergencyOverride::new(
            GovernanceAction::BlacklistNode {
                node_id: "bad_node".to_string(),
                reason: "Malicious".to_string(),
                duration: None,
            },
            "admin".to_string(),
            "Malicious behavior".to_string(),
        );

        let id = engine.submit_override(override_req).unwrap();

        // Need second approval for multi-sig
        engine.approve_override(&id, "admin2").unwrap();

        // Now execute
        engine.execute_overrides();

        assert!(engine.is_blacklisted("bad_node"));
        assert!(!engine.is_blacklisted("good_node"));
    }
}
