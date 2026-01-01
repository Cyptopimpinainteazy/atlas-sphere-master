// ============================================================================
// X3 ATLAS SPHERE - EXTENDED GOVERNANCE FRAMEWORK
// Regional Compliance, Content Sensitivity, Enhanced Auditing, Legal Framework
// ============================================================================

use crate::marketing_governance::{ComplianceCheck, AuditLogEntry, RateLimit};
use crate::swarm_core::{Language, Region};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ============================================================================
// REGIONAL COMPLIANCE FRAMEWORKS
// ============================================================================

/// Regulatory framework for a region
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegulatoryFramework {
    pub region: Region,
    pub frameworks: Vec<Regulation>,
    pub data_residency_required: bool,
    pub data_residency_countries: Vec<String>,
    pub consent_type: ConsentType,
    pub retention_period_days: u32,
    pub user_rights: UserRights,
    pub content_restrictions: ContentRestrictions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Regulation {
    pub name: String,                      // "GDPR", "CCPA", "LGPD", "PDPA"
    pub jurisdiction: String,
    pub effective_date: DateTime<Utc>,
    pub enforcement_level: EnforcementLevel,
    pub requirements: Vec<RegulatoryRequirement>,
    pub penalties: PenaltyStructure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnforcementLevel {
    Advisory,    // Nice to have
    Recommended, // Should do
    Required,    // Must do (legal requirement)
    Critical,    // Enforced with significant penalties
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegulatoryRequirement {
    pub requirement_id: Uuid,
    pub name: String,
    pub description: String,
    pub enforcement_level: EnforcementLevel,
    pub implementation: ImplementationGuidance,
    pub verification_method: VerificationMethod,
    pub deadline: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplementationGuidance {
    pub description: String,
    pub steps: Vec<String>,
    pub documentation_required: Vec<String>,
    pub estimated_effort: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationMethod {
    Automated,
    ManualReview,
    AuditLog,
    UserConsent,
    Documentation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PenaltyStructure {
    pub minor_violation: String,    // 1-5% of revenue
    pub major_violation: String,    // 5-20% of revenue
    pub critical_violation: String, // Up to 20+ million or % of revenue
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsentType {
    Explicit,       // Opt-in (GDPR, PDPA)
    OptOut,         // Opt-out (CCPA, limited)
    Legitimate,     // Legitimate interest (GDPR)
    Contractual,    // Performance of contract
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserRights {
    pub right_to_access: bool,
    pub right_to_rectification: bool,
    pub right_to_erasure: bool,
    pub right_to_restriction: bool,
    pub right_to_data_portability: bool,
    pub right_to_object: bool,
    pub right_to_not_be_subject_to_automated_decision: bool,
    pub right_to_explanation: bool,
    pub response_deadline_days: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContentRestrictions {
    pub prohibited_content_types: Vec<String>,
    pub require_age_gate: bool,
    pub minimum_age: Option<u8>,
    pub restricted_targeting: Vec<String>,
    pub forbidden_claims: Vec<String>,
}

// ============================================================================
// MAJOR COMPLIANCE FRAMEWORKS
// ============================================================================

impl RegulatoryFramework {
    /// GDPR (General Data Protection Regulation) - EU
    pub fn gdpr() -> Self {
        Self {
            region: Region::Europe,
            frameworks: vec![Regulation {
                name: "GDPR".to_string(),
                jurisdiction: "EU".to_string(),
                effective_date: DateTime::parse_from_rfc3339("2018-05-25T00:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
                enforcement_level: EnforcementLevel::Critical,
                requirements: vec![
                    RegulatoryRequirement {
                        requirement_id: Uuid::new_v4(),
                        name: "Lawful Basis".to_string(),
                        description: "Must have lawful basis for processing personal data".to_string(),
                        enforcement_level: EnforcementLevel::Critical,
                        implementation: ImplementationGuidance {
                            description: "Implement explicit consent mechanism".to_string(),
                            steps: vec![
                                "Implement opt-in consent".to_string(),
                                "Provide clear privacy policy".to_string(),
                                "Track consent records".to_string(),
                            ],
                            documentation_required: vec!["Privacy Policy".to_string(), "Consent Records".to_string()],
                            estimated_effort: "High".to_string(),
                        },
                        verification_method: VerificationMethod::UserConsent,
                        deadline: None,
                    },
                    RegulatoryRequirement {
                        requirement_id: Uuid::new_v4(),
                        name: "Data Subject Rights".to_string(),
                        description: "Respect user rights to access, rectify, erase, port data".to_string(),
                        enforcement_level: EnforcementLevel::Critical,
                        implementation: ImplementationGuidance {
                            description: "Implement data request system".to_string(),
                            steps: vec![
                                "Create user portal for data requests".to_string(),
                                "Establish 30-day response SLA".to_string(),
                                "Implement data export format".to_string(),
                            ],
                            documentation_required: vec!["Data Request Process".to_string()],
                            estimated_effort: "High".to_string(),
                        },
                        verification_method: VerificationMethod::AuditLog,
                        deadline: None,
                    },
                    RegulatoryRequirement {
                        requirement_id: Uuid::new_v4(),
                        name: "Data Protection Impact Assessment".to_string(),
                        description: "Conduct DPIA for high-risk processing".to_string(),
                        enforcement_level: EnforcementLevel::Required,
                        implementation: ImplementationGuidance {
                            description: "Document processing impact".to_string(),
                            steps: vec!["Document data flows".to_string(), "Assess risks".to_string(), "Implement controls".to_string()],
                            documentation_required: vec!["DPIA Document".to_string()],
                            estimated_effort: "Medium".to_string(),
                        },
                        verification_method: VerificationMethod::Documentation,
                        deadline: None,
                    },
                ],
                penalties: PenaltyStructure {
                    minor_violation: "Up to €10 million".to_string(),
                    major_violation: "Up to €20 million or 4% of revenue".to_string(),
                    critical_violation: "Up to €20 million or 4% of revenue".to_string(),
                },
            }],
            data_residency_required: true,
            data_residency_countries: vec!["EU".to_string()],
            consent_type: ConsentType::Explicit,
            retention_period_days: 365,
            user_rights: UserRights {
                right_to_access: true,
                right_to_rectification: true,
                right_to_erasure: true,
                right_to_restriction: true,
                right_to_data_portability: true,
                right_to_object: true,
                right_to_not_be_subject_to_automated_decision: true,
                right_to_explanation: true,
                response_deadline_days: 30,
            },
            content_restrictions: ContentRestrictions::default(),
        }
    }

    /// CCPA (California Consumer Privacy Act)
    pub fn ccpa() -> Self {
        Self {
            region: Region::NorthAmerica,
            frameworks: vec![Regulation {
                name: "CCPA".to_string(),
                jurisdiction: "California".to_string(),
                effective_date: DateTime::parse_from_rfc3339("2020-01-01T00:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
                enforcement_level: EnforcementLevel::Critical,
                requirements: vec![
                    RegulatoryRequirement {
                        requirement_id: Uuid::new_v4(),
                        name: "Privacy Notice".to_string(),
                        description: "Provide clear privacy notices".to_string(),
                        enforcement_level: EnforcementLevel::Critical,
                        implementation: ImplementationGuidance {
                            description: "Publish comprehensive privacy policy".to_string(),
                            steps: vec!["Draft privacy policy".to_string(), "Publish on website".to_string(), "Update annually".to_string()],
                            documentation_required: vec!["Privacy Policy".to_string()],
                            estimated_effort: "Medium".to_string(),
                        },
                        verification_method: VerificationMethod::Documentation,
                        deadline: None,
                    },
                ],
                penalties: PenaltyStructure {
                    minor_violation: "$2,500 per violation".to_string(),
                    major_violation: "$7,500 per intentional violation".to_string(),
                    critical_violation: "Class action lawsuits possible".to_string(),
                },
            }],
            data_residency_required: false,
            data_residency_countries: vec![],
            consent_type: ConsentType::OptOut,
            retention_period_days: 365,
            user_rights: UserRights {
                right_to_access: true,
                right_to_deletion: true,
                right_to_opt_out: true,
                response_deadline_days: 45,
                ..Default::default()
            },
            content_restrictions: ContentRestrictions::default(),
        }
    }

    /// LGPD (Lei Geral de Proteção de Dados) - Brazil
    pub fn lgpd() -> Self {
        Self {
            region: Region::LatinAmerica,
            frameworks: vec![Regulation {
                name: "LGPD".to_string(),
                jurisdiction: "Brazil".to_string(),
                effective_date: DateTime::parse_from_rfc3339("2020-09-18T00:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
                enforcement_level: EnforcementLevel::Critical,
                requirements: vec![],
                penalties: PenaltyStructure {
                    minor_violation: "Up to R$50 million per violation".to_string(),
                    major_violation: "Up to R$50 million per violation".to_string(),
                    critical_violation: "Up to R$50 million + 2% of revenue".to_string(),
                },
            }],
            data_residency_required: false,
            data_residency_countries: vec![],
            consent_type: ConsentType::Explicit,
            retention_period_days: 365,
            user_rights: UserRights {
                right_to_access: true,
                right_to_rectification: true,
                right_to_erasure: true,
                response_deadline_days: 15,
                ..Default::default()
            },
            content_restrictions: ContentRestrictions::default(),
        }
    }

    /// PDPA (Personal Data Protection Act) - Thailand/Singapore
    pub fn pdpa() -> Self {
        Self {
            region: Region::SoutheastAsia,
            frameworks: vec![Regulation {
                name: "PDPA".to_string(),
                jurisdiction: "Thailand/Southeast Asia".to_string(),
                effective_date: DateTime::parse_from_rfc3339("2020-06-01T00:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
                enforcement_level: EnforcementLevel::Critical,
                requirements: vec![],
                penalties: PenaltyStructure {
                    minor_violation: "Up to 5 million baht".to_string(),
                    major_violation: "Up to 5 million baht + imprisonment".to_string(),
                    critical_violation: "Up to 5 million baht + 5 years imprisonment".to_string(),
                },
            }],
            data_residency_required: true,
            data_residency_countries: vec!["Thailand".to_string(), "Singapore".to_string()],
            consent_type: ConsentType::Explicit,
            retention_period_days: 365,
            user_rights: UserRights {
                right_to_access: true,
                right_to_rectification: true,
                right_to_erasure: true,
                response_deadline_days: 30,
                ..Default::default()
            },
            content_restrictions: ContentRestrictions::default(),
        }
    }
}

// ============================================================================
// CONTENT SENSITIVITY MATRIX
// ============================================================================

/// Classifies content sensitivity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SensitivityLevel {
    Green,    // Safe for all audiences
    Yellow,   // Requires review
    Orange,   // Requires approval
    Red,      // Likely needs modification
    Black,    // Cannot post as-is
}

/// Content sensitivity classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentSensitivity {
    pub content_id: Uuid,
    pub overall_level: SensitivityLevel,
    pub categories: HashMap<SensitivityCategory, SensitivityScore>,
    pub flags: Vec<SensitivityFlag>,
    pub requires_review: bool,
    pub requires_approval: bool,
    pub requires_disclosure: bool,
    pub approval_status: ApprovalStatus,
    pub approved_by: Option<Uuid>,
    pub approved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SensitivityCategory {
    Violence,
    Adult,
    Hateful,
    Misleading,
    Spam,
    PrivateInfo,
    Regulatory,
    Political,
    Religious,
    Medical,
    Financial,
    Promotional,
    Discriminatory,
    Misinformation,
    Copyright,
    Impersonation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitivityScore {
    pub category: SensitivityCategory,
    pub score: f32,  // 0.0 to 1.0
    pub confidence: f32,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitivityFlag {
    pub flag_id: Uuid,
    pub category: SensitivityCategory,
    pub severity: Severity,
    pub description: String,
    pub suggested_action: SuggestedAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Info,
    Warning,
    Critical,
    BlockingIssue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuggestedAction {
    NoAction,
    Monitor,
    Review,
    Modify,
    Reject,
    ApprovalRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
    ModificationsNeeded,
    Escalated,
}

impl ContentSensitivity {
    pub fn new(content_id: Uuid) -> Self {
        Self {
            content_id,
            overall_level: SensitivityLevel::Green,
            categories: HashMap::new(),
            flags: Vec::new(),
            requires_review: false,
            requires_approval: false,
            requires_disclosure: false,
            approval_status: ApprovalStatus::Pending,
            approved_by: None,
            approved_at: None,
        }
    }

    pub fn calculate_overall_level(&mut self) {
        if self.flags.iter().any(|f| f.severity == Severity::BlockingIssue) {
            self.overall_level = SensitivityLevel::Black;
        } else if self.flags.iter().any(|f| f.severity == Severity::Critical) {
            self.overall_level = SensitivityLevel::Red;
        } else if self.categories.values().any(|s| s.score > 0.7) {
            self.overall_level = SensitivityLevel::Orange;
        } else if self.categories.values().any(|s| s.score > 0.4) {
            self.overall_level = SensitivityLevel::Yellow;
        } else {
            self.overall_level = SensitivityLevel::Green;
        }

        // Determine if review/approval needed
        self.requires_review = self.overall_level != SensitivityLevel::Green;
        self.requires_approval = self.overall_level == SensitivityLevel::Orange
            || self.overall_level == SensitivityLevel::Red;
    }

    pub fn can_publish(&self) -> bool {
        self.overall_level != SensitivityLevel::Black
            && (self.overall_level != SensitivityLevel::Red || self.approval_status == ApprovalStatus::Approved)
    }
}

// ============================================================================
// ENHANCED AUDIT SYSTEM
// ============================================================================

/// Comprehensive audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedAuditTrail {
    pub trail_id: Uuid,
    pub entity_id: Uuid,
    pub entity_type: AuditEntityType,
    pub entries: Vec<EnhancedAuditEntry>,
    pub retention_until: DateTime<Utc>,
    pub is_sealed: bool,
    pub seal_signature: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditEntityType {
    Content,
    User,
    Account,
    Campaign,
    Agent,
    ComplianceCheck,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedAuditEntry {
    pub entry_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub action: AuditAction,
    pub actor_id: Option<Uuid>,
    pub actor_type: ActorType,
    pub status: ActionStatus,
    pub changes: Vec<Change>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub reasoning: Option<String>,
    pub compliance_check: Option<String>,
    pub approvals: Vec<Approval>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditAction {
    Created,
    Modified,
    Deleted,
    Published,
    Scheduled,
    Rejected,
    Approved,
    Escalated,
    Archived,
    Exported,
    AccessRequested,
    DataDeleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActorType {
    User,
    Agent,
    System,
    API,
    Automation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionStatus {
    Success,
    Pending,
    Failed,
    RequiresApproval,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Change {
    pub field: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub change_type: ChangeType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    Created,
    Updated,
    Deleted,
    Published,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Approval {
    pub approver_id: Uuid,
    pub approved_at: DateTime<Utc>,
    pub approval_type: ApprovalType,
    pub notes: Option<String>,
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalType {
    Compliance,
    Legal,
    Management,
    Regulatory,
    Security,
}

// ============================================================================
// DISCLOSURE REQUIREMENT MANAGER
// ============================================================================

/// Manages AI disclosure requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisclosureRequirementManager {
    pub manager_id: Uuid,
    pub requirements_by_region: HashMap<Region, DisclosureRequirement>,
    pub requirements_by_platform: HashMap<String, DisclosureRequirement>,
    pub global_requirements: DisclosureRequirement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisclosureRequirement {
    pub required: bool,
    pub minimum_placement: DisclosurePlacement,
    pub minimum_prominence: DisclosureProminence,
    pub required_text: String,
    pub acceptable_alternatives: Vec<String>,
    pub language_specific: bool,
    pub example_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisclosurePlacement {
    Beginning,
    End,
    Prominent,
    BelowTheFold,
    AtLink,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisclosureProminence {
    Hidden,
    Low,
    Medium,
    High,
    VeryHigh,
}

impl DisclosureRequirementManager {
    pub fn new() -> Self {
        Self {
            manager_id: Uuid::new_v4(),
            requirements_by_region: HashMap::new(),
            requirements_by_platform: HashMap::new(),
            global_requirements: DisclosureRequirement {
                required: true,
                minimum_placement: DisclosurePlacement::Prominent,
                minimum_prominence: DisclosureProminence::High,
                required_text: "This content was generated with AI assistance".to_string(),
                acceptable_alternatives: vec![
                    "AI-assisted content".to_string(),
                    "Generated with AI".to_string(),
                    "[AI]".to_string(),
                ],
                language_specific: false,
                example_text: "🤖 AI-assisted: This content was created with the help of AI tools.".to_string(),
            },
        }
    }

    pub fn get_requirement(&self, region: Option<&Region>, platform: Option<&str>) -> &DisclosureRequirement {
        if let Some(platform) = platform {
            if let Some(req) = self.requirements_by_platform.get(platform) {
                return req;
            }
        }

        if let Some(region) = region {
            if let Some(req) = self.requirements_by_region.get(region) {
                return req;
            }
        }

        &self.global_requirements
    }

    pub fn check_disclosure(&self, content: &str, region: Option<&Region>, platform: Option<&str>) -> DisclosureCheck {
        let requirement = self.get_requirement(region, platform);

        let has_disclosure = content.to_lowercase().contains("ai")
            || content.contains("🤖")
            || requirement.acceptable_alternatives
                .iter()
                .any(|alt| content.contains(alt));

        DisclosureCheck {
            check_id: Uuid::new_v4(),
            is_compliant: has_disclosure || !requirement.required,
            has_disclosure,
            requirement: requirement.clone(),
            suggested_disclosure: if !has_disclosure {
                Some(requirement.example_text.clone())
            } else {
                None
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisclosureCheck {
    pub check_id: Uuid,
    pub is_compliant: bool,
    pub has_disclosure: bool,
    pub requirement: DisclosureRequirement,
    pub suggested_disclosure: Option<String>,
}

// ============================================================================
// COMPLIANCE DASHBOARD
// ============================================================================

/// Compliance health metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceDashboard {
    pub generated_at: DateTime<Utc>,
    pub overall_compliance_score: f32,
    pub regional_compliance: HashMap<Region, RegionalComplianceScore>,
    pub framework_compliance: HashMap<String, f32>,
    pub recent_issues: Vec<ComplianceIssue>,
    pub violations: Vec<ComplianceViolation>,
    pub action_items: Vec<ActionItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionalComplianceScore {
    pub region: String,
    pub score: f32,
    pub status: ComplianceStatus,
    pub frameworks: HashMap<String, f32>,
    pub last_audit: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComplianceStatus {
    Compliant,
    PartiallyCompliant,
    NonCompliant,
    UnderReview,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceIssue {
    pub issue_id: Uuid,
    pub severity: ComplianceSeverity,
    pub framework: String,
    pub description: String,
    pub detected_at: DateTime<Utc>,
    pub resolution_deadline: DateTime<Utc>,
    pub resolution_plan: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComplianceSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceViolation {
    pub violation_id: Uuid,
    pub framework: String,
    pub violation_type: String,
    pub detected_at: DateTime<Utc>,
    pub remediation_steps: Vec<String>,
    pub remediation_deadline: DateTime<Utc>,
    pub status: ViolationStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViolationStatus {
    Reported,
    InRemidiation,
    Resolved,
    Escalated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionItem {
    pub item_id: Uuid,
    pub priority: ActionPriority,
    pub framework: String,
    pub requirement: String,
    pub due_date: DateTime<Utc>,
    pub owner: String,
    pub status: ActionItemStatus,
    pub progress: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionItemStatus {
    NotStarted,
    InProgress,
    Blocked,
    OnTrack,
    OffTrack,
    Completed,
}

impl ComplianceDashboard {
    pub fn new() -> Self {
        Self {
            generated_at: Utc::now(),
            overall_compliance_score: 0.0,
            regional_compliance: HashMap::new(),
            framework_compliance: HashMap::new(),
            recent_issues: Vec::new(),
            violations: Vec::new(),
            action_items: Vec::new(),
        }
    }

    pub fn calculate_overall_score(&mut self) {
        if self.framework_compliance.is_empty() {
            self.overall_compliance_score = 0.0;
            return;
        }

        let total: f32 = self.framework_compliance.values().sum();
        self.overall_compliance_score = total / self.framework_compliance.len() as f32;
    }

    pub fn get_critical_items(&self) -> Vec<&ComplianceIssue> {
        self.recent_issues
            .iter()
            .filter(|i| i.severity == ComplianceSeverity::Critical)
            .collect()
    }

    pub fn get_overdue_actions(&self) -> Vec<&ActionItem> {
        self.action_items
            .iter()
            .filter(|a| a.due_date < Utc::now() && a.status != ActionItemStatus::Completed)
            .collect()
    }
}

// ============================================================================
// UNIT TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gdpr_framework() {
        let gdpr = RegulatoryFramework::gdpr();
        assert_eq!(gdpr.region, Region::Europe);
        assert_eq!(gdpr.consent_type, ConsentType::Explicit);
        assert!(gdpr.user_rights.right_to_erasure);
    }

    #[test]
    fn test_ccpa_framework() {
        let ccpa = RegulatoryFramework::ccpa();
        assert_eq!(ccpa.consent_type, ConsentType::OptOut);
    }

    #[test]
    fn test_content_sensitivity() {
        let mut sensitivity = ContentSensitivity::new(Uuid::new_v4());
        sensitivity.categories.insert(
            SensitivityCategory::Violence,
            SensitivityScore {
                category: SensitivityCategory::Violence,
                score: 0.8,
                confidence: 0.95,
                reason: "Contains violent imagery".to_string(),
            },
        );
        sensitivity.calculate_overall_level();

        assert_eq!(sensitivity.overall_level, SensitivityLevel::Red);
        assert!(sensitivity.requires_approval);
    }

    #[test]
    fn test_disclosure_manager() {
        let manager = DisclosureRequirementManager::new();
        let content = "This is a normal post 🤖";

        let check = manager.check_disclosure(&content, None, None);
        assert!(check.has_disclosure);
        assert!(check.is_compliant);
    }

    #[test]
    fn test_compliance_dashboard() {
        let mut dashboard = ComplianceDashboard::new();
        dashboard.framework_compliance.insert("GDPR".to_string(), 0.9);
        dashboard.framework_compliance.insert("CCPA".to_string(), 0.85);

        dashboard.calculate_overall_score();
        assert!(dashboard.overall_compliance_score > 0.8);
    }

    #[test]
    fn test_audit_trail_creation() {
        let trail = EnhancedAuditTrail {
            trail_id: Uuid::new_v4(),
            entity_id: Uuid::new_v4(),
            entity_type: AuditEntityType::Content,
            entries: vec![],
            retention_until: Utc::now() + Duration::days(365),
            is_sealed: false,
            seal_signature: None,
        };

        assert_eq!(trail.entity_type, AuditEntityType::Content);
        assert!(!trail.is_sealed);
    }
}
