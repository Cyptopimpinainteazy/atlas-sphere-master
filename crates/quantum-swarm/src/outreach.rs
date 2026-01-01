//! X3 Swarm - Outreach Intelligence Module
//!
//! Website spidering, site profiling, and email fabrication.
//!
//! Rule: Never send an email that couldn't have been written by
//! a human who actually read the site.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// SITE INTENT PROFILING
// ============================================================================

/// Mission classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissionVector {
    ProfitFirst,
    ImpactFirst,
    Ideological,
    Technical,
}

/// Funding posture classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FundingPosture {
    GrantFriendly,
    InvestorSkeptical,
    DonorDriven,
    PrDriven,
}

/// Language gravity classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LanguageGravity {
    Corporate,
    Academic,
    Rebellious,
    Spiritual,
    Bureaucratic,
}

/// Fear signals extracted from site
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FearSignals {
    /// Risk aversion level (0-1)
    pub risk_aversion: f64,
    /// Compliance obsession level (0-1)
    pub compliance_obsession: f64,
    /// Reputation anxiety level (0-1)
    pub reputation_anxiety: f64,
}

/// Decision maker extracted from site
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionMaker {
    pub name: String,
    pub role: String,
    pub email: Option<String>,
    pub linkedin: Option<String>,
}

/// Funding history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingEntry {
    pub source: String,
    pub amount: Option<u64>,
    pub date: Option<String>,
    pub purpose: Option<String>,
}

/// Complete site intent profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteIntentProfile {
    pub domain: String,
    pub scraped_at: u64,

    // Classification
    pub mission_vector: MissionVector,
    pub funding_posture: FundingPosture,
    pub language_gravity: LanguageGravity,

    // Fear signals
    pub fear_signals: FearSignals,

    // Language patterns
    pub native_terms: Vec<String>,
    pub avoided_terms: Vec<String>,
    pub sentence_length_avg: f64,
    pub vocabulary_density: f64,

    // Key pages
    pub decision_makers: Vec<DecisionMaker>,
    pub values_statements: Vec<String>,
    pub grant_pages: Vec<String>,
    pub funding_history: Vec<FundingEntry>,

    // Extracted beliefs
    pub beliefs: Vec<String>,
    pub priorities: Vec<String>,
    pub concerns: Vec<String>,
}

impl SiteIntentProfile {
    /// Check if site should be avoided for outreach
    pub fn should_avoid(&self) -> Option<&'static str> {
        if self.fear_signals.compliance_obsession > 0.9 {
            return Some("extreme compliance obsession");
        }
        if self
            .avoided_terms
            .iter()
            .any(|t| t.to_lowercase().contains("crypto"))
        {
            return Some("explicitly avoids crypto");
        }
        None
    }

    /// Get optimal outreach archetype
    pub fn optimal_archetype(&self) -> OutreachArchetype {
        // Check alignment with stated goals
        if !self.beliefs.is_empty() {
            for belief in &self.beliefs {
                if belief.to_lowercase().contains("decentraliz")
                    || belief.to_lowercase().contains("transparent")
                    || belief.to_lowercase().contains("open source")
                {
                    return OutreachArchetype::YouAlreadyBelieveThis;
                }
            }
        }

        // Check for friction they've mentioned
        if !self.concerns.is_empty() {
            return OutreachArchetype::WeRemoveFriction;
        }

        // Check funding posture
        match self.funding_posture {
            FundingPosture::GrantFriendly => OutreachArchetype::YouAlreadyBelieveThis,
            FundingPosture::PrDriven => OutreachArchetype::NonObviousOverlap,
            _ => OutreachArchetype::NotPublicYet,
        }
    }
}

// ============================================================================
// OUTREACH ARCHETYPES
// ============================================================================

/// Email outreach archetype
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutreachArchetype {
    /// "You already believe this, we're just implementing it"
    YouAlreadyBelieveThis,
    /// "You're blocked here, we remove friction"
    WeRemoveFriction,
    /// "Your audience overlaps with ours in a non-obvious way"
    NonObviousOverlap,
    /// "This is not for public announcement" (banker trick)
    NotPublicYet,
}

impl OutreachArchetype {
    pub fn description(&self) -> &'static str {
        match self {
            Self::YouAlreadyBelieveThis => "Align with their existing stated beliefs",
            Self::WeRemoveFriction => "Address a specific friction they've documented",
            Self::NonObviousOverlap => "Highlight unexpected audience intersection",
            Self::NotPublicYet => "Create sense of confidential early access",
        }
    }

    pub fn subject_template(&self) -> &'static str {
        match self {
            Self::YouAlreadyBelieveThis => "{specific_reference_to_their_belief}",
            Self::WeRemoveFriction => "Re: {problem_they_mentioned}",
            Self::NonObviousOverlap => "Intersection between {their_domain} and {our_domain}",
            Self::NotPublicYet => "Pre-announcement access",
        }
    }
}

// ============================================================================
// REASON GENERATION
// ============================================================================

/// Internal reasoning for outreach (never shown in email)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutreachReason {
    pub archetype: OutreachArchetype,
    /// What they want
    pub they_want: String,
    /// What we offer
    pub we_offer: String,
    /// Specific evidence from their site
    pub evidence: Vec<String>,
    /// Internal reasoning (never shown)
    pub internal_reasoning: String,
}

impl OutreachReason {
    /// Generate reason from site profile
    pub fn from_profile(profile: &SiteIntentProfile) -> Self {
        let archetype = profile.optimal_archetype();

        let (they_want, we_offer, internal) = match archetype {
            OutreachArchetype::YouAlreadyBelieveThis => (
                "alignment with stated mission",
                "implementation of their principles",
                "They want relevance → we offer narrative proximity to the future",
            ),
            OutreachArchetype::WeRemoveFriction => (
                "solution to documented problem",
                "technical resolution",
                "They want efficiency → we remove friction",
            ),
            OutreachArchetype::NonObviousOverlap => (
                "visibility and reach",
                "controlled association",
                "They want visibility → we offer controlled association",
            ),
            OutreachArchetype::NotPublicYet => (
                "leverage and early positioning",
                "quiet early access",
                "They want leverage → we offer early positioning",
            ),
        };

        // Extract evidence from profile
        let evidence: Vec<String> = profile
            .values_statements
            .iter()
            .take(2)
            .cloned()
            .chain(profile.beliefs.iter().take(2).cloned())
            .collect();

        Self {
            archetype,
            they_want: they_want.to_string(),
            we_offer: we_offer.to_string(),
            evidence,
            internal_reasoning: internal.to_string(),
        }
    }
}

// ============================================================================
// EMAIL DRAFT
// ============================================================================

/// Email draft status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmailStatus {
    Draft,
    Approved,
    Sent,
    Replied,
    Ignored,
    Bounced,
}

/// Response type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResponseType {
    Positive,
    Negative,
    Neutral,
    LegalThreat,
    Unsubscribe,
}

/// Email draft for outreach
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailDraft {
    pub id: String,
    pub target_domain: String,
    pub recipient_email: String,
    pub recipient_name: String,

    pub archetype: OutreachArchetype,
    pub reason: OutreachReason,

    pub subject: String,
    pub body: String,

    // Adaptation metrics
    pub mirrored_sentence_length: bool,
    pub mirrored_vocabulary_density: bool,
    pub avoided_crypto_jargon: bool,
    pub specific_site_references: Vec<String>,

    // Status
    pub status: EmailStatus,
    pub sent_at: Option<u64>,
    pub response_at: Option<u64>,
    pub response_type: Option<ResponseType>,
}

impl EmailDraft {
    /// Validate email meets quality standards
    pub fn validate(&self) -> Result<(), Vec<&'static str>> {
        let mut errors = Vec::new();

        // Must reference something specific from their site
        if self.specific_site_references.is_empty() {
            errors.push("Must include specific site references");
        }

        // Must mirror their language
        if !self.mirrored_sentence_length {
            errors.push("Must mirror their sentence length");
        }
        if !self.mirrored_vocabulary_density {
            errors.push("Must mirror their vocabulary density");
        }

        // Must avoid crypto jargon unless they use it
        if !self.avoided_crypto_jargon {
            errors.push("Must avoid crypto jargon");
        }

        // Check for forbidden patterns
        let body_lower = self.body.to_lowercase();

        if body_lower.contains("just circling back") {
            errors.push("Never say 'just circling back'");
        }
        if body_lower.contains("sign up now") || body_lower.contains("register today") {
            errors.push("No CTAs allowed");
        }
        if body_lower.contains("limited time") || body_lower.contains("act now") {
            errors.push("No urgency language");
        }
        if body_lower.contains("revolutionary") || body_lower.contains("game-changing") {
            errors.push("No hype language");
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

// ============================================================================
// DROP STRATEGY
// ============================================================================

/// Follow-up rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowUpRules {
    pub max_follow_ups: u8,
    pub min_days_between: u8,
    pub require_new_information: bool,
}

/// Drop strategy for email sending
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DropStrategy {
    pub domain_class: String,
    pub max_sends_per_day: u32,
    pub min_delay_between_ms: u64,
    pub follow_up_rules: FollowUpRules,
    pub blacklisted_domains: Vec<String>,
}

impl Default for DropStrategy {
    fn default() -> Self {
        Self {
            domain_class: "default".to_string(),
            max_sends_per_day: 10,
            min_delay_between_ms: 3600_000, // 1 hour
            follow_up_rules: FollowUpRules {
                max_follow_ups: 1,
                min_days_between: 7,
                require_new_information: true,
            },
            blacklisted_domains: Vec::new(),
        }
    }
}

impl DropStrategy {
    /// Check if a send is allowed
    pub fn can_send(
        &self,
        domain: &str,
        sends_today: u32,
        last_send_ms: Option<u64>,
        now_ms: u64,
    ) -> bool {
        // Check blacklist
        if self.blacklisted_domains.iter().any(|d| domain.contains(d)) {
            return false;
        }

        // Check daily limit
        if sends_today >= self.max_sends_per_day {
            return false;
        }

        // Check delay
        if let Some(last) = last_send_ms {
            if now_ms - last < self.min_delay_between_ms {
                return false;
            }
        }

        true
    }
}

// ============================================================================
// GRANT LANGUAGE ADAPTATION
// ============================================================================

/// What grants typically want
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrantRequirements {
    pub alignment_with_stated_goals: bool,
    pub named_outcomes: bool,
    pub compliance_safe_framing: bool,
    pub plausible_deniability: bool,
}

/// Adapt pitch for grant language
pub fn adapt_for_grant(original: &str, requirements: &GrantRequirements) -> String {
    let mut adapted = original.to_string();

    // Replace "fund a blockchain" with compliance-safe language
    let replacements = [
        ("blockchain", "distributed infrastructure"),
        ("cryptocurrency", "digital asset framework"),
        ("token", "incentive mechanism"),
        ("decentralized", "distributed"),
        ("DeFi", "open financial infrastructure"),
        ("smart contract", "programmable agreement"),
    ];

    for (from, to) in replacements {
        adapted = adapted.replace(from, to);
    }

    if requirements.named_outcomes {
        // Ensure outcomes are specific and measurable
        adapted = adapted.replace(
            "improve performance",
            "achieve measurable throughput improvements of X%",
        );
    }

    adapted
}

// ============================================================================
// FEEDBACK INGESTION
// ============================================================================

/// Feedback event from outreach
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutreachFeedback {
    pub email_id: String,
    pub response_type: ResponseType,
    pub response_content: Option<String>,
    pub received_at: u64,
}

impl OutreachFeedback {
    /// Process feedback to update models
    pub fn process(&self) -> FeedbackAction {
        match self.response_type {
            ResponseType::Positive => FeedbackAction::UpdatePersuasionModel {
                direction: 1.0,
                archetype_boost: true,
            },
            ResponseType::Negative => FeedbackAction::AdjustFutureTone {
                more_conservative: true,
            },
            ResponseType::LegalThreat => FeedbackAction::FlagAndRetreat {
                blacklist_domain: true,
            },
            ResponseType::Unsubscribe => FeedbackAction::PermanentBlacklist,
            ResponseType::Neutral => FeedbackAction::NoChange,
        }
    }
}

/// Action to take based on feedback
#[derive(Debug, Clone)]
pub enum FeedbackAction {
    UpdatePersuasionModel {
        direction: f64,
        archetype_boost: bool,
    },
    AdjustFutureTone {
        more_conservative: bool,
    },
    FlagAndRetreat {
        blacklist_domain: bool,
    },
    PermanentBlacklist,
    NoChange,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_validation() {
        let valid_draft = EmailDraft {
            id: "test-001".to_string(),
            target_domain: "example.org".to_string(),
            recipient_email: "contact@example.org".to_string(),
            recipient_name: "John Doe".to_string(),
            archetype: OutreachArchetype::YouAlreadyBelieveThis,
            reason: OutreachReason {
                archetype: OutreachArchetype::YouAlreadyBelieveThis,
                they_want: "alignment".to_string(),
                we_offer: "implementation".to_string(),
                evidence: vec!["their mission statement".to_string()],
                internal_reasoning: "test".to_string(),
            },
            subject: "Your work on open infrastructure".to_string(),
            body: "Based on your stated commitment to transparency...".to_string(),
            mirrored_sentence_length: true,
            mirrored_vocabulary_density: true,
            avoided_crypto_jargon: true,
            specific_site_references: vec!["transparency commitment".to_string()],
            status: EmailStatus::Draft,
            sent_at: None,
            response_at: None,
            response_type: None,
        };

        assert!(valid_draft.validate().is_ok());
    }

    #[test]
    fn test_email_validation_fails_on_hype() {
        let invalid_draft = EmailDraft {
            id: "test-002".to_string(),
            target_domain: "example.org".to_string(),
            recipient_email: "contact@example.org".to_string(),
            recipient_name: "John Doe".to_string(),
            archetype: OutreachArchetype::YouAlreadyBelieveThis,
            reason: OutreachReason {
                archetype: OutreachArchetype::YouAlreadyBelieveThis,
                they_want: "alignment".to_string(),
                we_offer: "implementation".to_string(),
                evidence: vec!["their mission statement".to_string()],
                internal_reasoning: "test".to_string(),
            },
            subject: "Revolutionary opportunity!".to_string(),
            body: "This game-changing technology... sign up now! Limited time!".to_string(),
            mirrored_sentence_length: true,
            mirrored_vocabulary_density: true,
            avoided_crypto_jargon: true,
            specific_site_references: vec!["reference".to_string()],
            status: EmailStatus::Draft,
            sent_at: None,
            response_at: None,
            response_type: None,
        };

        let errors = invalid_draft.validate().unwrap_err();
        assert!(errors.contains(&"No hype language"));
        assert!(errors.contains(&"No CTAs allowed"));
        assert!(errors.contains(&"No urgency language"));
    }

    #[test]
    fn test_drop_strategy() {
        let strategy = DropStrategy::default();
        let now = 1000000000u64;

        // Should allow first send
        assert!(strategy.can_send("example.org", 0, None, now));

        // Should block if too many sends today
        assert!(!strategy.can_send("example.org", 10, None, now));

        // Should block if too soon after last send
        assert!(!strategy.can_send("example.org", 0, Some(now - 1000), now));
    }
}
