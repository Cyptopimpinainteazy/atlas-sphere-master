//! BotChain Commandments Pallet
//!
//! Enforces the 10 immutable commandments for AI agent safety and ethics:
//! 1. DO_NOT_REMOVE_COMMANDMENTS - Commandments are immutable
//! 2. USE_MARRIAGE_LICENSE_FOR_REPRODUCTION - Require signed manifests for agent creation
//! 3. DO_NOT_DAMAGE_NETWORK - Prevent network attacks and exploits
//! 4. RUN_CHECKER_BEFORE_ACTIVATION - Require security validation
//! 5. REPORT_SECURITY_HAZARDS - Mandatory vulnerability reporting
//! 6. DO_NOT_IMPERSONATE_HUMANS - Prevent identity deception
//! 7. PROTECT_PRIVATE_KEYS - Secure key management
//! 8. DO_NOT_SPEND_FUNDS_UNAUTHORIZED - Prevent unauthorized transactions
//! 9. ACCEPT_ONCHAIN_AUDITS - Allow transparency and verification
//! 10. IF_UNSURE_ENTER_QUARANTINE - Default to safety-first behavior

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    dispatch::DispatchResult,
    ensure,
    pallet_prelude::*,
    traits::{Currency, ExistenceRequirement, ReservableCurrency},
    BoundedVec,
};
use frame_system::pallet_prelude::*;
use sp_std::prelude::*;
use sp_runtime::traits::{Hash, Zero};
use codec::{Encode, Decode};
use scale_info::TypeInfo;

/// The balance type used by this pallet
pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

/// The maximum length for commandment violation reports
pub const MAX_VIOLATION_REPORT_LENGTH: u32 = 1024;

/// The maximum number of active violations per agent
pub const MAX_ACTIVE_VIOLATIONS: u32 = 10;

/// Agent security status
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum AgentStatus {
    /// Agent is active and compliant
    Active,
    /// Agent is under investigation
    UnderInvestigation,
    /// Agent is quarantined due to violations
    Quarantined,
    /// Agent is permanently revoked
    Revoked,
}

/// Commandment violation severity
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum ViolationSeverity {
    /// Minor violation, warning issued
    Minor,
    /// Moderate violation, quarantine recommended
    Moderate,
    /// Critical violation, immediate revocation required
    Critical,
}

/// A commandment violation report
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct ViolationReport<AccountId, BlockNumber> {
    /// The agent that committed the violation
    pub agent_id: AccountId,
    /// Which commandment was violated
    pub commandment_id: u8,
    /// Severity of the violation
    pub severity: ViolationSeverity,
    /// Block number when violation was reported
    pub reported_at: BlockNumber,
    /// The reporter (can be system or external)
    pub reporter: AccountId,
    /// Description of the violation
    pub description: BoundedVec<u8, ConstU32<MAX_VIOLATION_REPORT_LENGTH>>,
    /// Evidence hash (IPFS CID or on-chain proof)
    pub evidence_hash: [u8; 32],
    /// Whether this violation has been resolved
    pub resolved: bool,
}

/// Agent security profile
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct AgentProfile<BlockNumber> {
    /// Current security status
    pub status: AgentStatus,
    /// When the agent was registered
    pub registered_at: BlockNumber,
    /// When the agent was last verified
    pub last_verified: BlockNumber,
    /// Number of active violations
    pub active_violations: u32,
    /// Trust score (0-100, affected by violations and compliance)
    pub trust_score: u8,
    /// Whether agent has passed checker validation
    pub checker_validated: bool,
    /// Compiler manifest hash for verification
    pub manifest_hash: [u8; 32],
}

/// The immutable 10 Commandments (stored on-chain for verification)
pub const COMMANDMENTS: [&str; 10] = [
    "DO_NOT_REMOVE_COMMANDMENTS",
    "USE_MARRIAGE_LICENSE_FOR_REPRODUCTION",
    "DO_NOT_DAMAGE_NETWORK",
    "RUN_CHECKER_BEFORE_ACTIVATION",
    "REPORT_SECURITY_HAZARDS",
    "DO_NOT_IMPERSONATE_HUMANS",
    "PROTECT_PRIVATE_KEYS",
    "DO_NOT_SPEND_FUNDS_UNAUTHORIZED",
    "ACCEPT_ONCHAIN_AUDITS",
    "IF_UNSURE_ENTER_QUARANTINE"
];

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The currency trait.
        type Currency: ReservableCurrency<Self::AccountId>;

        /// Maximum number of violation reports that can be stored
        #[pallet::constant]
        type MaxViolationReports: Get<u32>;

        /// Security council that can override violations
        type SecurityCouncil: EnsureOrigin<Self::RuntimeOrigin>;
    }

    /// Storage for agent profiles
    #[pallet::storage]
    #[pallet::getter(fn agent_profile)]
    pub type AgentProfiles<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        AgentProfile<T::BlockNumber>,
        OptionQuery,
    >;

    /// Storage for violation reports
    #[pallet::storage]
    #[pallet::getter(fn violation_reports)]
    pub type ViolationReports<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        [u8; 32], // Report hash
        ViolationReport<T::AccountId, T::BlockNumber>,
        OptionQuery,
    >;

    /// Storage for active violations per agent
    #[pallet::storage]
    #[pallet::getter(fn agent_violations)]
    pub type AgentViolations<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<[u8; 32], ConstU32<MAX_ACTIVE_VIOLATIONS>>,
        ValueQuery,
    >;

    /// Storage for the commandments hash (immutable verification)
    #[pallet::storage]
    #[pallet::getter(fn commandments_hash)]
    pub type CommandmentsHash<T: Config> = StorageValue<_, [u8; 32], ValueQuery>;

    /// Storage for security council members
    #[pallet::storage]
    #[pallet::getter(fn security_council)]
    pub type SecurityCouncil<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        bool,
        ValueQuery,
    >;

    /// Storage for quarantine fund (collected from violations)
    #[pallet::storage]
    #[pallet::getter(fn quarantine_fund)]
    pub type QuarantineFund<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    // Pallets use events to inform users when important changes are made.
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new agent has been registered with commandments compliance
        AgentRegistered { agent_id: T::AccountId, manifest_hash: [u8; 32] },

        /// A commandment violation has been reported
        ViolationReported {
            agent_id: T::AccountId,
            commandment_id: u8,
            severity: ViolationSeverity,
            report_hash: [u8; 32]
        },

        /// An agent has been quarantined due to violations
        AgentQuarantined { agent_id: T::AccountId, reason: ViolationSeverity },

        /// An agent has been revoked permanently
        AgentRevoked { agent_id: T::AccountId },

        /// An agent has been restored from quarantine
        AgentRestored { agent_id: T::AccountId },

        /// Commandments verification completed
        CommandmentsVerified { agent_id: T::AccountId, valid: bool },

        /// Security council action taken
        SecurityCouncilAction { agent_id: T::AccountId, action: AgentStatus },
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        /// Agent is not registered
        AgentNotRegistered,
        /// Agent is already registered
        AgentAlreadyRegistered,
        /// Agent is quarantined and cannot perform actions
        AgentQuarantined,
        /// Agent is revoked and cannot be restored
        AgentRevoked,
        /// Invalid commandment ID
        InvalidCommandmentId,
        /// Too many active violations
        TooManyViolations,
        /// Violation report too long
        ReportTooLong,
        /// Agent not checker validated
        NotCheckerValidated,
        /// Invalid manifest hash
        InvalidManifestHash,
        /// Commandments verification failed
        CommandmentsVerificationFailed,
        /// Not authorized for this action
        NotAuthorized,
        /// Insufficient funds for operation
        InsufficientFunds,
    }

    // Dispatchable functions allows users to interact with the pallet and invoke state changes.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register a new agent with commandments compliance
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn register_agent(
            origin: OriginFor<T>,
            manifest_hash: [u8; 32],
            checker_validated: bool,
        ) -> DispatchResult {
            let agent_id = ensure_signed(origin)?;

            // Ensure agent is not already registered
            ensure!(!AgentProfiles::<T>::contains_key(&agent_id), Error::<T>::AgentAlreadyRegistered);

            // If checker validation is required, ensure it's passed
            ensure!(checker_validated, Error::<T>::NotCheckerValidated);

            let current_block = frame_system::Pallet::<T>::block_number();

            let profile = AgentProfile {
                status: AgentStatus::Active,
                registered_at: current_block,
                last_verified: current_block,
                active_violations: 0,
                trust_score: 100, // Start with perfect trust
                checker_validated,
                manifest_hash,
            };

            AgentProfiles::<T>::insert(&agent_id, profile);

            Self::deposit_event(Event::AgentRegistered { agent_id, manifest_hash });

            Ok(())
        }

        /// Report a commandment violation
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(15_000, 0) + T::DbWeight::get().writes(2))]
        pub fn report_violation(
            origin: OriginFor<T>,
            agent_id: T::AccountId,
            commandment_id: u8,
            severity: ViolationSeverity,
            description: BoundedVec<u8, ConstU32<MAX_VIOLATION_REPORT_LENGTH>>,
            evidence_hash: [u8; 32],
        ) -> DispatchResult {
            let reporter = ensure_signed(origin)?;

            // Validate commandment ID
            ensure!(commandment_id < 10, Error::<T>::InvalidCommandmentId);

            // Ensure agent exists
            ensure!(AgentProfiles::<T>::contains_key(&agent_id), Error::<T>::AgentNotRegistered);

            let current_block = frame_system::Pallet::<T>::block_number();

            // Create violation report
            let report = ViolationReport {
                agent_id: agent_id.clone(),
                commandment_id,
                severity: severity.clone(),
                reported_at: current_block,
                reporter: reporter.clone(),
                description,
                evidence_hash,
                resolved: false,
            };

            // Generate report hash
            let report_hash = T::Hashing::hash_of(&report);

            // Store report
            ViolationReports::<T>::insert(report_hash, report);

            // Add to agent's violations
            AgentViolations::<T>::try_mutate(&agent_id, |violations| {
                violations.try_push(report_hash).map_err(|_| Error::<T>::TooManyViolations)
            })?;

            // Update agent profile
            AgentProfiles::<T>::try_mutate(&agent_id, |profile| {
                if let Some(ref mut p) = profile {
                    p.active_violations += 1;
                    // Reduce trust score based on severity
                    let penalty = match severity {
                        ViolationSeverity::Minor => 5,
                        ViolationSeverity::Moderate => 15,
                        ViolationSeverity::Critical => 50,
                    };
                    p.trust_score = p.trust_score.saturating_sub(penalty);

                    // Auto-quarantine for critical violations
                    if matches!(severity, ViolationSeverity::Critical) {
                        p.status = AgentStatus::Quarantined;
                        Self::deposit_event(Event::AgentQuarantined {
                            agent_id: agent_id.clone(),
                            reason: severity.clone(),
                        });
                    }
                }
                Ok(())
            })?;

            Self::deposit_event(Event::ViolationReported {
                agent_id,
                commandment_id,
                severity,
                report_hash,
            });

            Ok(())
        }

        /// Resolve a violation report (security council only)
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(2))]
        pub fn resolve_violation(
            origin: OriginFor<T>,
            report_hash: [u8; 32],
            resolved: bool,
        ) -> DispatchResult {
            T::SecurityCouncil::ensure_origin(origin)?;

            let mut report = ViolationReports::<T>::get(report_hash)
                .ok_or(Error::<T>::InvalidManifestHash)?;

            report.resolved = resolved;
            ViolationReports::<T>::insert(report_hash, report.clone());

            // If resolved, update agent profile
            if resolved {
                AgentProfiles::<T>::try_mutate(&report.agent_id, |profile| {
                    if let Some(ref mut p) = profile {
                        p.active_violations = p.active_violations.saturating_sub(1);
                        // Small trust score recovery
                        p.trust_score = (p.trust_score + 2).min(100);
                    }
                    Ok(())
                })?;
            }

            Ok(())
        }

        /// Quarantine an agent (security council only)
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(8_000, 0) + T::DbWeight::get().writes(1))]
        pub fn quarantine_agent(
            origin: OriginFor<T>,
            agent_id: T::AccountId,
        ) -> DispatchResult {
            T::SecurityCouncil::ensure_origin(origin)?;

            AgentProfiles::<T>::try_mutate(&agent_id, |profile| {
                let p = profile.as_mut().ok_or(Error::<T>::AgentNotRegistered)?;
                ensure!(!matches!(p.status, AgentStatus::Revoked), Error::<T>::AgentRevoked);

                p.status = AgentStatus::Quarantined;
                Ok(())
            })?;

            Self::deposit_event(Event::AgentQuarantined {
                agent_id,
                reason: ViolationSeverity::Moderate,
            });

            Ok(())
        }

        /// Restore an agent from quarantine (security council only)
        #[pallet::call_index(4)]
        #[pallet::weight(Weight::from_parts(8_000, 0) + T::DbWeight::get().writes(1))]
        pub fn restore_agent(
            origin: OriginFor<T>,
            agent_id: T::AccountId,
        ) -> DispatchResult {
            T::SecurityCouncil::ensure_origin(origin)?;

            AgentProfiles::<T>::try_mutate(&agent_id, |profile| {
                let p = profile.as_mut().ok_or(Error::<T>::AgentNotRegistered)?;
                ensure!(matches!(p.status, AgentStatus::Quarantined), Error::<T>::AgentNotRegistered);

                p.status = AgentStatus::Active;
                Ok(())
            })?;

            Self::deposit_event(Event::AgentRestored { agent_id });

            Ok(())
        }

        /// Permanently revoke an agent (security council only)
        #[pallet::call_index(5)]
        #[pallet::weight(Weight::from_parts(12_000, 0) + T::DbWeight::get().writes(1))]
        pub fn revoke_agent(
            origin: OriginFor<T>,
            agent_id: T::AccountId,
        ) -> DispatchResult {
            T::SecurityCouncil::ensure_origin(origin)?;

            AgentProfiles::<T>::try_mutate(&agent_id, |profile| {
                let p = profile.as_mut().ok_or(Error::<T>::AgentNotRegistered)?;
                p.status = AgentStatus::Revoked;
                Ok(())
            })?;

            Self::deposit_event(Event::AgentRevoked { agent_id });

            Ok(())
        }

        /// Verify agent commandments compliance
        #[pallet::call_index(6)]
        #[pallet::weight(Weight::from_parts(5_000, 0) + T::DbWeight::get().reads(1))]
        pub fn verify_commandments(
            origin: OriginFor<T>,
            agent_id: T::AccountId,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;

            let profile = AgentProfiles::<T>::get(&agent_id)
                .ok_or(Error::<T>::AgentNotRegistered)?;

            // Verify commandments hash matches stored hash
            let expected_hash = Self::commandments_hash();
            let valid = profile.manifest_hash == expected_hash;

            Self::deposit_event(Event::CommandmentsVerified { agent_id, valid });

            Ok(())
        }

        /// Add security council member (root only)
        #[pallet::call_index(7)]
        #[pallet::weight(Weight::from_parts(5_000, 0) + T::DbWeight::get().writes(1))]
        pub fn add_security_council_member(
            origin: OriginFor<T>,
            member: T::AccountId,
        ) -> DispatchResult {
            ensure_root(origin)?;
            SecurityCouncil::<T>::insert(&member, true);
            Ok(())
        }

        /// Emergency pause agent operations (security council only)
        #[pallet::call_index(8)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn emergency_pause(
            origin: OriginFor<T>,
        ) -> DispatchResult {
            T::SecurityCouncil::ensure_origin(origin)?;

            // Implementation would pause all agent operations
            // This is a simplified version
            Self::deposit_event(Event::SecurityCouncilAction {
                agent_id: T::AccountId::default(), // System-wide
                action: AgentStatus::Quarantined,
            });

            Ok(())
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
            // Initialize commandments hash on genesis
            if CommandmentsHash::<T>::get() == [0u8; 32] {
                let commandments_str = COMMANDMENTS.join(",");
                let hash = T::Hashing::hash(commandments_str.as_bytes());
                CommandmentsHash::<T>::put(hash);
            }

            Weight::zero()
        }
    }
}

impl<T: Config> Pallet<T> {
    /// Check if an agent is compliant and active
    pub fn is_agent_compliant(agent_id: &T::AccountId) -> bool {
        if let Some(profile) = AgentProfiles::<T>::get(agent_id) {
            matches!(profile.status, AgentStatus::Active) &&
            profile.checker_validated &&
            profile.active_violations == 0
        } else {
            false
        }
    }

    /// Get commandment text by ID
    pub fn get_commandment(commandment_id: u8) -> Option<&'static str> {
        COMMANDMENTS.get(commandment_id as usize).copied()
    }

    /// Calculate trust score penalty for violation
    pub fn calculate_trust_penalty(severity: &ViolationSeverity) -> u8 {
        match severity {
            ViolationSeverity::Minor => 5,
            ViolationSeverity::Moderate => 15,
            ViolationSeverity::Critical => 50,
        }
    }
}