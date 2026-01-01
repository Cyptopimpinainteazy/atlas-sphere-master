/// Founder Media Engine: Contributor Framework
///
/// System for managing real humans as swarm assets:
/// - Written consent + usage scope
/// - Revocation rights (explicit control)
/// - Compensation tracking
/// - Disclosure audit trail
/// - Content derivative rights
///
/// Everything is licensed, not stolen. Everything is auditable.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc, Duration};

/// A real human with explicit consent to be featured in content
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Contributor {
    /// Unique ID (UUID or username)
    pub id: String,

    /// Legal name
    pub name: String,

    /// Public name/brand name (what appears in videos)
    pub public_name: String,

    /// Email for consent tracking
    pub email: String,

    /// Wallet address (for payments/equity)
    pub wallet: Option<String>,

    /// Role type
    pub role: ContributorRole,

    /// Status (active, paused, revoked, retired)
    pub status: ContributorStatus,

    /// When was this contributor on-boarded?
    pub created_at: DateTime<Utc>,

    /// Last updated
    pub updated_at: DateTime<Utc>,
}

/// What kind of work does this person do?
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ContributorRole {
    /// Primary founder/anchor identity
    Founder,

    /// Technical presenter/educator
    Educator,

    /// Narrator/voice talent
    Narrator,

    /// On-camera presenter
    Presenter,

    /// Community host/manager
    CommunityHost,

    /// Expert guest/interviewer
    GuestExpert,

    /// Producer/crew
    Producer,
}

/// Is this person available to be used?
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContributorStatus {
    /// Active and ready to use
    Active,

    /// Temporarily paused (vacation, sabbatical, etc)
    Paused,

    /// Permanently revoked (do not use their likeness)
    Revoked,

    /// Retired (archived, historical reference only)
    Retired,
}

/// Explicit consent to use someone's likeness/voice
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContributorConsent {
    /// Who is giving consent?
    pub contributor_id: String,

    /// Version of consent (multiple agreements possible)
    pub version: u32,

    /// What can you do with this person's likeness?
    pub permitted_uses: Vec<UsageScope>,

    /// What are you NOT allowed to do?
    pub prohibited_uses: Vec<String>,

    /// Who is allowed to use this content?
    pub licensee_scope: LicenseeScope,

    /// Geographic restrictions
    pub geographic_scope: GeographicScope,

    /// Time limits (consent expires?)
    pub duration: ConsentDuration,

    /// Compensation type
    pub compensation: CompensationType,

    /// When was this agreed to?
    pub signed_at: DateTime<Utc>,

    /// Hash of the signed agreement (for verification)
    pub agreement_hash: String,

    /// Can they revoke this? (always true, by law)
    pub revocable: bool,

    /// Has it been revoked?
    pub revoked_at: Option<DateTime<Utc>>,
}

/// What is someone's likeness allowed to be used for?
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum UsageScope {
    /// Recorded once, used in videos
    RecordedContent,

    /// Dubbed into other languages
    DubLocalization,

    /// Split into clips and shorts
    ClippingDerivatives,

    /// Repurposed into educational content
    EducationalDerivatives,

    /// Used in social media posts
    SocialMediaDistribution,

    /// Used in paid advertising
    PaidAdvertising,

    /// Used in live streams (requires real-time consent)
    LiveStreaming,

    /// Used for AI training (synthetic voice, synthetic video)
    AiTraining,

    /// Used in merchandise or physical products
    Merchandise,

    /// Used in presentations/keynotes
    PublicPresentation,
}

/// Who is allowed to use this content?
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LicenseeScope {
    /// Only the specified entity
    SpecificEntity(String),

    /// The organization + approved partners
    OrganizationAndPartners,

    /// Public (anyone, with attribution)
    Public,

    /// Internal only (not public)
    Internal,
}

/// Where can this be used?
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GeographicScope {
    /// Worldwide
    Global,

    /// Only specific regions
    Regions(Vec<String>),

    /// Only specific countries
    Countries(Vec<String>),

    /// Only specific platforms
    Platforms(Vec<String>),
}

/// How long is consent valid?
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ConsentDuration {
    /// Until explicitly revoked
    Indefinite,

    /// Expires on specific date
    UntilDate(DateTime<Utc>),

    /// Valid for N years
    Years(u32),

    /// Valid for specific number of uses
    UsageLimit(u32),
}

/// How is the contributor paid?
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CompensationType {
    /// One-time flat fee
    FlatFee {
        amount: String, // denominated in USD or crypto
        currency: String,
    },

    /// Per usage (e.g., per video, per 1000 views)
    PerUsage {
        amount: String,
        unit: String, // "per_video", "per_1000_views", etc
    },

    /// Equity in the project/company
    Equity {
        percentage: f64,
        cliff_months: Option<u32>,
        vesting_months: Option<u32>,
    },

    /// Revenue sharing
    RevenueShare {
        percentage: f64,
    },

    /// Combination
    Hybrid {
        base_fee: String,
        performance_bonus: Option<String>,
        equity_percentage: Option<f64>,
    },

    /// Volunteer/unpaid
    Volunteer,
}

/// Track actual usage for payment/audit purposes
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContributorUsageRecord {
    /// Which consent agreement was used?
    pub consent_id: String,

    /// Who was used?
    pub contributor_id: String,

    /// What content was created?
    pub content_asset_id: String,

    /// What usage scope was this? (String for flexibility)
    pub usage_scope: String,

    /// Timestamp of usage
    pub used_at: DateTime<Utc>,

    /// Details (which video, which language, which platform)
    pub context: String,

    /// Should this contributor be paid for this usage?
    pub compensable: bool,
}

/// Manager for all contributors and their licensing
pub struct ContributorManager {
    contributors: HashMap<String, Contributor>,
    consents: HashMap<String, ContributorConsent>,
    usage_records: Vec<ContributorUsageRecord>,
}

impl ContributorManager {
    pub fn new() -> Self {
        Self {
            contributors: HashMap::new(),
            consents: HashMap::new(),
            usage_records: Vec::new(),
        }
    }

    /// Register a new contributor
    pub fn register_contributor(&mut self, contributor: Contributor) -> Result<String, String> {
        let id = contributor.id.clone();
        if self.contributors.contains_key(&id) {
            return Err(format!("Contributor {} already registered", id));
        }
        self.contributors.insert(id.clone(), contributor);
        Ok(id)
    }

    /// Get a contributor
    pub fn get_contributor(&self, id: &str) -> Option<&Contributor> {
        self.contributors.get(id)
    }

    /// Create consent agreement
    pub fn create_consent(&mut self, consent: ContributorConsent) -> Result<String, String> {
        // Verify contributor exists
        if !self.contributors.contains_key(&consent.contributor_id) {
            return Err(format!(
                "Contributor {} not found",
                consent.contributor_id
            ));
        }

        let consent_id = format!("{}-v{}-{}", consent.contributor_id, consent.version, uuid::Uuid::new_v4());
        self.consents.insert(consent_id.clone(), consent);
        Ok(consent_id)
    }

    /// Check if a contributor can be used for a specific purpose
    pub fn can_use_for(
        &self,
        contributor_id: &str,
        usage: UsageScope,
    ) -> Result<bool, String> {
        // Get latest active consent (if none, treat as not permitted)
        let consent_opt = self
            .consents
            .values()
            .filter(|c| c.contributor_id == contributor_id && c.revoked_at.is_none())
            .max_by_key(|c| c.version);

        let consent = match consent_opt {
            Some(c) => c,
            None => return Ok(false),
        };

        // Check if usage is permitted
        if !consent.permitted_uses.contains(&usage) {
            return Ok(false);
        }

        // Check if revoked
        if consent.revoked_at.is_some() {
            return Ok(false);
        }

        // Check if expired
        let now = Utc::now();
        let expired = match &consent.duration {
            ConsentDuration::Indefinite => false,
            ConsentDuration::UntilDate(date) => now > *date,
            ConsentDuration::Years(years) => {
                now > consent.signed_at + Duration::days(365 * *years as i64)
            }
            ConsentDuration::UsageLimit(_) => false, // Check separately
        };

        Ok(!expired)
    }

    /// Record a contributor's usage (for audit trail + payment)
    pub fn record_usage(&mut self, record: ContributorUsageRecord) {
        self.usage_records.push(record);
    }

    /// Revoke consent (explicit control)
    pub fn revoke_consent(&mut self, consent_id: &str) -> Result<(), String> {
        self.consents
            .get_mut(consent_id)
            .ok_or_else(|| "Consent not found".to_string())?
            .revoked_at = Some(Utc::now());
        Ok(())
    }

    /// Pause a contributor (temporarily unavailable)
    pub fn pause_contributor(&mut self, contributor_id: &str) -> Result<(), String> {
        self.contributors
            .get_mut(contributor_id)
            .ok_or_else(|| "Contributor not found".to_string())?
            .status = ContributorStatus::Paused;
        Ok(())
    }

    /// Resume a contributor
    pub fn resume_contributor(&mut self, contributor_id: &str) -> Result<(), String> {
        self.contributors
            .get_mut(contributor_id)
            .ok_or_else(|| "Contributor not found".to_string())?
            .status = ContributorStatus::Active;
        Ok(())
    }

    /// Get usage history for a contributor (for payment tracking)
    pub fn get_usage_history(&self, contributor_id: &str) -> Vec<&ContributorUsageRecord> {
        self.usage_records
            .iter()
            .filter(|r| r.contributor_id == contributor_id)
            .collect()
    }

    /// Get all contributors
    pub fn list_contributors(&self) -> Vec<&Contributor> {
        self.contributors.values().collect()
    }

    /// Get active contributors by role
    pub fn get_by_role(&self, role: ContributorRole) -> Vec<&Contributor> {
        self.contributors
            .values()
            .filter(|c| c.role == role && c.status == ContributorStatus::Active)
            .collect()
    }
}

impl Default for ContributorManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contributor_registration() {
        let mut manager = ContributorManager::new();
        let contributor = Contributor {
            id: "alice".to_string(),
            name: "Alice Smith".to_string(),
            public_name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
            wallet: None,
            role: ContributorRole::Educator,
            status: ContributorStatus::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let result = manager.register_contributor(contributor);
        assert!(result.is_ok());

        let retrieved = manager.get_contributor("alice");
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_consent_creation() {
        let mut manager = ContributorManager::new();
        let contributor = Contributor {
            id: "bob".to_string(),
            name: "Bob Jones".to_string(),
            public_name: "Bob".to_string(),
            email: "bob@example.com".to_string(),
            wallet: Some("0x123...".to_string()),
            role: ContributorRole::Presenter,
            status: ContributorStatus::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        manager.register_contributor(contributor).unwrap();

        let consent = ContributorConsent {
            contributor_id: "bob".to_string(),
            version: 1,
            permitted_uses: vec![UsageScope::RecordedContent, UsageScope::SocialMediaDistribution],
            prohibited_uses: vec!["Live streaming without notice".to_string()],
            licensee_scope: LicenseeScope::OrganizationAndPartners,
            geographic_scope: GeographicScope::Global,
            duration: ConsentDuration::Indefinite,
            compensation: CompensationType::FlatFee {
                amount: "5000".to_string(),
                currency: "USD".to_string(),
            },
            signed_at: Utc::now(),
            agreement_hash: "0xabcd...".to_string(),
            revocable: true,
            revoked_at: None,
        };

        let result = manager.create_consent(consent);
        assert!(result.is_ok());
    }

    #[test]
    fn test_usage_permission_check() {
        let mut manager = ContributorManager::new();
        let contributor = Contributor {
            id: "charlie".to_string(),
            name: "Charlie Brown".to_string(),
            public_name: "Charlie".to_string(),
            email: "charlie@example.com".to_string(),
            wallet: None,
            role: ContributorRole::Narrator,
            status: ContributorStatus::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        manager.register_contributor(contributor).unwrap();

        let consent = ContributorConsent {
            contributor_id: "charlie".to_string(),
            version: 1,
            permitted_uses: vec![UsageScope::RecordedContent, UsageScope::DubLocalization],
            prohibited_uses: vec![],
            licensee_scope: LicenseeScope::Public,
            geographic_scope: GeographicScope::Global,
            duration: ConsentDuration::Indefinite,
            compensation: CompensationType::Volunteer,
            signed_at: Utc::now(),
            agreement_hash: "0xefgh...".to_string(),
            revocable: true,
            revoked_at: None,
        };

        manager.create_consent(consent).unwrap();

        // Should allow recording
        assert!(manager
            .can_use_for("charlie", UsageScope::RecordedContent)
            .unwrap());

        // Should allow dubbing
        assert!(manager
            .can_use_for("charlie", UsageScope::DubLocalization)
            .unwrap());

        // Should NOT allow live streaming (not in permitted uses)
        assert!(!manager
            .can_use_for("charlie", UsageScope::LiveStreaming)
            .unwrap());
    }

    #[test]
    fn test_revocation() {
        let mut manager = ContributorManager::new();
        let contributor = Contributor {
            id: "dave".to_string(),
            name: "Dave Wilson".to_string(),
            public_name: "Dave".to_string(),
            email: "dave@example.com".to_string(),
            wallet: None,
            role: ContributorRole::GuestExpert,
            status: ContributorStatus::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        manager.register_contributor(contributor).unwrap();

        let consent = ContributorConsent {
            contributor_id: "dave".to_string(),
            version: 1,
            permitted_uses: vec![UsageScope::RecordedContent],
            prohibited_uses: vec![],
            licensee_scope: LicenseeScope::Public,
            geographic_scope: GeographicScope::Global,
            duration: ConsentDuration::Indefinite,
            compensation: CompensationType::Volunteer,
            signed_at: Utc::now(),
            agreement_hash: "0xijkl...".to_string(),
            revocable: true,
            revoked_at: None,
        };

        let consent_id = manager.create_consent(consent).unwrap();

        // Sanity check: consent inserted and active
        // Debug print all consent entries (test-only)
        for (id, c) in manager.consents.iter() {
            println!("consent entry: {} -> {} revoked_at={:?} version={}", id, c.contributor_id, c.revoked_at, c.version);
        }
        let matches: Vec<_> = manager.consents.values().filter(|c| c.contributor_id == "dave" && c.revoked_at.is_none()).collect();
        println!("matching active consents: {}", matches.len());
        if !matches.is_empty() {
            println!("first match version={}", matches[0].version);
        }
        assert!(manager.consents.values().any(|c| c.contributor_id == "dave" && c.revoked_at.is_none()));

        // Should work before revocation
        assert!(manager
            .can_use_for("dave", UsageScope::RecordedContent)
            .unwrap());

        // Revoke consent
        manager.revoke_consent(&consent_id).unwrap();

        // Should NOT work after revocation
        assert!(!manager
            .can_use_for("dave", UsageScope::RecordedContent)
            .unwrap());
    }
}
