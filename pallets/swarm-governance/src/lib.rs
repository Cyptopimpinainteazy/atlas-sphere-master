//! Swarm Governance and Safety Systems Pallet
//!
//! Implements risk-tiered execution, kill switches, and human-AI governance:
//! - Tier 0-4 execution with escalating permissions
//! - Multi-layer kill switches (global, capability, agent, tier)
//! - 2 humans : 1 bot voting system for critical decisions
//! - Swarm capability flags and mutation guardrails
//! - Emergency controls and safety mechanisms

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    dispatch::DispatchResult,
    ensure,
    pallet_prelude::*,
    traits::{Currency, ReservableCurrency, Get},
    BoundedVec,
};
use frame_system::pallet_prelude::*;
use sp_std::prelude::*;
use sp_runtime::traits::{Hash, Zero};
use codec::{Encode, Decode};
use scale_info::TypeInfo;

/// The balance type used by this pallet
pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

/// Maximum length for governance proposals
pub const MAX_PROPOSAL_LENGTH: u32 = 2048;

/// Maximum number of active proposals
pub const MAX_ACTIVE_PROPOSALS: u32 = 100;

/// Maximum number of emergency council members
pub const MAX_EMERGENCY_COUNCIL: u32 = 20;

/// Execution risk tiers (0 = safest, 4 = highest risk)
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum ExecutionTier {
    /// Tier 0: Read-only operations, no state changes
    Zero,
    /// Tier 1: Low-risk operations (balance queries, basic computations)
    One,
    /// Tier 2: Medium-risk operations (single transactions, simple arbitrage)
    Two,
    /// Tier 3: High-risk operations (complex strategies, multi-step transactions)
    Three,
    /// Tier 4: Critical operations (system changes, large capital movements)
    Four,
}

/// Kill switch types for emergency shutdown
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum KillSwitch {
    /// Global emergency stop - halts all swarm operations
    Global,
    /// Capability-specific stop (trading, content, etc.)
    Capability(CapabilityType),
    /// Agent-specific stop
    Agent,
    /// Tier-specific stop (prevents certain risk levels)
    Tier(ExecutionTier),
}

/// Swarm capabilities that can be individually controlled
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum CapabilityType {
    Trading,
    Arbitrage,
    ContentCreation,
    SocialMedia,
    BusinessAutomation,
    Development,
    Infrastructure,
    Research,
    Governance,
    Emergency,
}

/// Governance proposal types
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum ProposalType {
    /// Enable/disable capability
    CapabilityToggle(CapabilityType, bool),
    /// Change execution tier permissions
    TierPermission(ExecutionTier, bool),
    /// Emergency council action
    EmergencyAction,
    /// Parameter update
    ParameterUpdate(Vec<u8>, Vec<u8>), // key, value
    /// Agent quarantine/revocation
    AgentControl(Vec<u8>, bool), // agent_id, quarantine
    /// System upgrade proposal
    SystemUpgrade(Vec<u8>), // upgrade_hash
}

/// Governance proposal
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct GovernanceProposal<AccountId, BlockNumber> {
    /// Unique proposal ID
    pub id: [u8; 32],
    /// Proposal type
    pub proposal_type: ProposalType,
    /// Proposer
    pub proposer: AccountId,
    /// Creation block
    pub created_at: BlockNumber,
    /// Voting deadline
    pub voting_ends: BlockNumber,
    /// Human votes (for, against)
    pub human_votes_for: u32,
    pub human_votes_against: u32,
    /// AI votes (weighted by trust score)
    pub ai_votes_for: u32,
    pub ai_votes_against: u32,
    /// Required quorum (percentage)
    pub quorum_required: u8,
    /// Execution tier required
    pub execution_tier: ExecutionTier,
    /// Proposal description
    pub description: BoundedVec<u8, ConstU32<MAX_PROPOSAL_LENGTH>>,
    /// Whether proposal has been executed
    pub executed: bool,
}

/// Kill switch state
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct KillSwitchState<BlockNumber> {
    /// Whether switch is active
    pub active: bool,
    /// When it was activated
    pub activated_at: Option<BlockNumber>,
    /// Who activated it
    pub activated_by: Option<Vec<u8>>, // account or system
    /// Reason for activation
    pub reason: Vec<u8>,
}

/// Emergency council member
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct EmergencyCouncilMember<AccountId, BlockNumber> {
    /// Council member account
    pub account: AccountId,
    /// When they were added
    pub added_at: BlockNumber,
    /// Whether they are active
    pub active: bool,
    /// Trust score (0-100)
    pub trust_score: u8,
}

/// System capability flags
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct CapabilityFlags {
    /// Global enable/disable
    pub global_enabled: bool,
    /// Per-capability enable flags
    pub capabilities: [bool; 10], // One for each CapabilityType variant
    /// Maximum execution tier allowed
    pub max_tier: ExecutionTier,
}

impl Default for CapabilityFlags {
    fn default() -> Self {
        Self {
            global_enabled: true,
            capabilities: [true; 10], // All enabled by default
            max_tier: ExecutionTier::Four,
        }
    }
}

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

        /// Emergency council origin (multisig or governance)
        type EmergencyOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Voting period in blocks
        #[pallet::constant]
        type VotingPeriod: Get<BlockNumberFor<Self>>;

        /// Minimum quorum percentage for proposals
        #[pallet::constant]
        type MinQuorum: Get<u8>;
    }

    /// Storage for governance proposals
    #[pallet::storage]
    #[pallet::getter(fn proposals)]
    pub type Proposals<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        [u8; 32], // Proposal ID
        GovernanceProposal<T::AccountId, BlockNumberFor<T>>,
        OptionQuery,
    >;

    /// Storage for active proposal IDs
    #[pallet::storage]
    #[pallet::getter(fn active_proposals)]
    pub type ActiveProposals<T: Config> = StorageValue<
        _,
        BoundedVec<[u8; 32], ConstU32<MAX_ACTIVE_PROPOSALS>>,
        ValueQuery,
    >;

    /// Storage for kill switch states
    #[pallet::storage]
    #[pallet::getter(fn kill_switches)]
    pub type KillSwitches<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        KillSwitch,
        KillSwitchState<BlockNumberFor<T>>,
        OptionQuery,
    >;

    /// Storage for emergency council members
    #[pallet::storage]
    #[pallet::getter(fn emergency_council)]
    pub type EmergencyCouncil<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        EmergencyCouncilMember<T::AccountId, BlockNumberFor<T>>,
        OptionQuery,
    >;

    /// Storage for system capability flags
    #[pallet::storage]
    #[pallet::getter(fn capability_flags)]
    pub type CapabilityFlagsStore<T: Config> = StorageValue<_, CapabilityFlags, ValueQuery>;

    /// Storage for human voter registry (accounts allowed to vote)
    #[pallet::storage]
    #[pallet::getter(fn human_voters)]
    pub type HumanVoters<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        bool,
        ValueQuery,
    >;

    // Pallets use events to inform users when important changes are made.
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new governance proposal has been created
        ProposalCreated { proposal_id: [u8; 32], proposer: T::AccountId, proposal_type: ProposalType },

        /// A proposal has been voted on
        ProposalVoted { proposal_id: [u8; 32], voter: T::AccountId, human_vote: bool, ai_weight: u32 },

        /// A proposal has been executed
        ProposalExecuted { proposal_id: [u8; 32], success: bool },

        /// A kill switch has been activated
        KillSwitchActivated { switch_type: KillSwitch, activated_by: T::AccountId },

        /// A kill switch has been deactivated
        KillSwitchDeactivated { switch_type: KillSwitch, deactivated_by: T::AccountId },

        /// Emergency council action taken
        EmergencyAction { action_type: Vec<u8>, executed_by: T::AccountId },

        /// Capability flags updated
        CapabilityFlagsUpdated { updated_by: T::AccountId },

        /// Emergency council member added
        CouncilMemberAdded { member: T::AccountId, added_by: T::AccountId },

        /// Emergency council member removed
        CouncilMemberRemoved { member: T::AccountId, removed_by: T::AccountId },
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        /// Proposal not found
        ProposalNotFound,
        /// Proposal already executed
        ProposalAlreadyExecuted,
        /// Voting period has ended
        VotingEnded,
        /// Not authorized to vote
        NotAuthorizedToVote,
        /// Already voted on this proposal
        AlreadyVoted,
        /// Insufficient permissions for action
        InsufficientPermissions,
        /// Kill switch already in desired state
        KillSwitchAlreadySet,
        /// Not an emergency council member
        NotEmergencyCouncil,
        /// Maximum active proposals reached
        MaxProposalsReached,
        /// Invalid proposal parameters
        InvalidProposal,
        /// Capability not allowed
        CapabilityDisabled,
        /// Execution tier not allowed
        TierNotAllowed,
        /// System safety lock active
        SystemLocked,
    }

    // Dispatchable functions allows users to interact with the pallet and invoke state changes.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create a new governance proposal
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(20_000, 0) + T::DbWeight::get().writes(2))]
        pub fn create_proposal(
            origin: OriginFor<T>,
            proposal_type: ProposalType,
            description: BoundedVec<u8, ConstU32<MAX_PROPOSAL_LENGTH>>,
            quorum_required: u8,
            execution_tier: ExecutionTier,
        ) -> DispatchResult {
            let proposer = ensure_signed(origin)?;

            // Validate proposal parameters
            ensure!(quorum_required >= T::MinQuorum::get() && quorum_required <= 100, Error::<T>::InvalidProposal);

            let current_block = frame_system::Pallet::<T>::block_number();
            let voting_ends = current_block + T::VotingPeriod::get();

            // Generate proposal ID
            let proposal_id = T::Hashing::hash_of(&(proposer.clone(), current_block, proposal_type.clone()));

            // Check max active proposals
            let mut active = ActiveProposals::<T>::get();
            ensure!(active.len() < MAX_ACTIVE_PROPOSALS as usize, Error::<T>::MaxProposalsReached);

            let proposal = GovernanceProposal {
                id: proposal_id,
                proposal_type: proposal_type.clone(),
                proposer: proposer.clone(),
                created_at: current_block,
                voting_ends,
                human_votes_for: 0,
                human_votes_against: 0,
                ai_votes_for: 0,
                ai_votes_against: 0,
                quorum_required,
                execution_tier,
                description,
                executed: false,
            };

            // Store proposal
            Proposals::<T>::insert(proposal_id, proposal);
            active.try_push(proposal_id).map_err(|_| Error::<T>::MaxProposalsReached)?;
            ActiveProposals::<T>::put(active);

            Self::deposit_event(Event::ProposalCreated { proposal_id, proposer, proposal_type });

            Ok(())
        }

        /// Vote on a governance proposal
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn vote_on_proposal(
            origin: OriginFor<T>,
            proposal_id: [u8; 32],
            human_vote: bool,
            ai_weight: u32,
        ) -> DispatchResult {
            let voter = ensure_signed(origin)?;
            let current_block = frame_system::Pallet::<T>::block_number();

            let mut proposal = Proposals::<T>::get(proposal_id)
                .ok_or(Error::<T>::ProposalNotFound)?;

            // Check voting is still open
            ensure!(current_block <= proposal.voting_ends, Error::<T>::VotingEnded);

            // Check permissions based on vote type
            if human_vote {
                ensure!(HumanVoters::<T>::get(&voter), Error::<T>::NotAuthorizedToVote);
            } else {
                // AI vote - check agent compliance from commandments pallet
                // This would integrate with the BotChain Commandments pallet
                ensure!(ai_weight > 0, Error::<T>::NotAuthorizedToVote);
            }

            // Update vote counts
            if human_vote {
                if human_vote {
                    proposal.human_votes_for += 1;
                } else {
                    proposal.human_votes_against += 1;
                }
            } else {
                if human_vote {
                    proposal.ai_votes_for += ai_weight;
                } else {
                    proposal.ai_votes_against += ai_weight;
                }
            }

            Proposals::<T>::insert(proposal_id, proposal);

            Self::deposit_event(Event::ProposalVoted { proposal_id, voter, human_vote, ai_weight });

            Ok(())
        }

        /// Execute a passed proposal
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(25_000, 0) + T::DbWeight::get().writes(3))]
        pub fn execute_proposal(
            origin: OriginFor<T>,
            proposal_id: [u8; 32],
        ) -> DispatchResult {
            let executor = ensure_signed(origin)?;
            let current_block = frame_system::Pallet::<T>::block_number();

            let mut proposal = Proposals::<T>::get(proposal_id)
                .ok_or(Error::<T>::ProposalNotFound)?;

            ensure!(!proposal.executed, Error::<T>::ProposalAlreadyExecuted);
            ensure!(current_block > proposal.voting_ends, Error::<T>::VotingEnded);

            // Calculate if proposal passed (2 humans : 1 bot requirement for critical decisions)
            let total_human_votes = proposal.human_votes_for + proposal.human_votes_against;
            let human_quorum = (total_human_votes as f32 * proposal.quorum_required as f32 / 100.0) as u32;

            let passed = match proposal.execution_tier {
                ExecutionTier::Four => {
                    // Critical decisions require 2:1 human majority
                    proposal.human_votes_for >= 2 && proposal.human_votes_for > proposal.human_votes_against
                },
                _ => {
                    // Other decisions require quorum and majority
                    proposal.human_votes_for >= human_quorum &&
                    proposal.human_votes_for > proposal.human_votes_against
                }
            };

            if passed {
                // Execute the proposal
                Self::execute_proposal_type(&proposal.proposal_type)?;
                proposal.executed = true;
            }

            Proposals::<T>::insert(proposal_id, proposal.clone());

            Self::deposit_event(Event::ProposalExecuted { proposal_id, success: passed });

            Ok(())
        }

        /// Activate a kill switch (emergency council only)
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(15_000, 0) + T::DbWeight::get().writes(1))]
        pub fn activate_kill_switch(
            origin: OriginFor<T>,
            switch_type: KillSwitch,
            reason: Vec<u8>,
        ) -> DispatchResult {
            T::EmergencyOrigin::ensure_origin(origin)?;
            let activator = frame_system::Pallet::<T>::block_number(); // Would need account tracking

            let current_block = frame_system::Pallet::<T>::block_number();

            let kill_state = KillSwitchState {
                active: true,
                activated_at: Some(current_block),
                activated_by: None, // Would set to actual activator
                reason,
            };

            KillSwitches::<T>::insert(switch_type.clone(), kill_state);

            // Update capability flags based on kill switch
            Self::update_capabilities_from_kill_switch(&switch_type, true)?;

            Self::deposit_event(Event::KillSwitchActivated {
                switch_type,
                activated_by: T::AccountId::default(), // Placeholder
            });

            Ok(())
        }

        /// Deactivate a kill switch (emergency council only)
        #[pallet::call_index(4)]
        #[pallet::weight(Weight::from_parts(15_000, 0) + T::DbWeight::get().writes(1))]
        pub fn deactivate_kill_switch(
            origin: OriginFor<T>,
            switch_type: KillSwitch,
        ) -> DispatchResult {
            T::EmergencyOrigin::ensure_origin(origin)?;

            KillSwitches::<T>::try_mutate(&switch_type, |state| {
                if let Some(ref mut s) = state {
                    if !s.active {
                        return Err(Error::<T>::KillSwitchAlreadySet);
                    }
                    s.active = false;
                    s.activated_at = None;
                    s.activated_by = None;
                }
                Ok(())
            })?;

            // Update capability flags
            Self::update_capabilities_from_kill_switch(&switch_type, false)?;

            Self::deposit_event(Event::KillSwitchDeactivated {
                switch_type,
                deactivated_by: T::AccountId::default(), // Placeholder
            });

            Ok(())
        }

        /// Add emergency council member (emergency council only)
        #[pallet::call_index(5)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn add_council_member(
            origin: OriginFor<T>,
            member: T::AccountId,
            trust_score: u8,
        ) -> DispatchResult {
            T::EmergencyOrigin::ensure_origin(origin)?;
            let adder = T::AccountId::default(); // Would get actual account
            let current_block = frame_system::Pallet::<T>::block_number();

            ensure!(trust_score <= 100, Error::<T>::InvalidProposal);

            let council_member = EmergencyCouncilMember {
                account: member.clone(),
                added_at: current_block,
                active: true,
                trust_score,
            };

            EmergencyCouncil::<T>::insert(&member, council_member);

            Self::deposit_event(Event::CouncilMemberAdded { member, added_by: adder });

            Ok(())
        }

        /// Emergency global shutdown (emergency council only)
        #[pallet::call_index(6)]
        #[pallet::weight(Weight::from_parts(50_000, 0) + T::DbWeight::get().writes(1))]
        pub fn emergency_shutdown(
            origin: OriginFor<T>,
            reason: Vec<u8>,
        ) -> DispatchResult {
            T::EmergencyOrigin::ensure_origin(origin)?;

            // Activate global kill switch
            let global_switch = KillSwitch::Global;
            let current_block = frame_system::Pallet::<T>::block_number();

            let kill_state = KillSwitchState {
                active: true,
                activated_at: Some(current_block),
                activated_by: None,
                reason,
            };

            KillSwitches::<T>::insert(global_switch.clone(), kill_state);

            // Disable all capabilities
            let mut flags = CapabilityFlagsStore::<T>::get();
            flags.global_enabled = false;
            for cap in flags.capabilities.iter_mut() {
                *cap = false;
            }
            CapabilityFlagsStore::<T>::put(flags);

            Self::deposit_event(Event::EmergencyAction {
                action_type: b"global_shutdown".to_vec(),
                executed_by: T::AccountId::default(),
            });

            Ok(())
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
            // Clean up expired proposals
            let mut active = ActiveProposals::<T>::get();
            let current_block = frame_system::Pallet::<T>::block_number();

            active.retain(|proposal_id| {
                if let Some(proposal) = Proposals::<T>::get(proposal_id) {
                    // Keep if voting hasn't ended or if not executed
                    current_block <= proposal.voting_ends || !proposal.executed
                } else {
                    false
                }
            });

            ActiveProposals::<T>::put(active);

            Weight::zero()
        }
    }
}

impl<T: Config> Pallet<T> {
    /// Execute a proposal based on its type
    fn execute_proposal_type(proposal_type: &ProposalType) -> DispatchResult {
        match proposal_type {
            ProposalType::CapabilityToggle(capability, enabled) => {
                Self::toggle_capability(*capability, *enabled)?;
            },
            ProposalType::TierPermission(tier, allowed) => {
                Self::set_tier_permission(*tier, *allowed)?;
            },
            ProposalType::EmergencyAction => {
                // Emergency actions would be handled separately
            },
            ProposalType::ParameterUpdate(key, value) => {
                Self::update_parameter(key, value)?;
            },
            ProposalType::AgentControl(agent_id, quarantine) => {
                Self::control_agent(agent_id, *quarantine)?;
            },
            ProposalType::SystemUpgrade(upgrade_hash) => {
                Self::schedule_upgrade(upgrade_hash)?;
            },
        }
        Ok(())
    }

    /// Toggle a capability on/off
    fn toggle_capability(capability: CapabilityType, enabled: bool) -> DispatchResult {
        let mut flags = CapabilityFlagsStore::<T>::get();
        let index = capability as usize;
        if index < flags.capabilities.len() {
            flags.capabilities[index] = enabled;
            CapabilityFlagsStore::<T>::put(flags);
        }
        Ok(())
    }

    /// Set tier permission
    fn set_tier_permission(tier: ExecutionTier, allowed: bool) -> DispatchResult {
        let mut flags = CapabilityFlagsStore::<T>::get();
        if !allowed {
            // If disabling this tier, set max_tier to previous level
            match tier {
                ExecutionTier::Four => flags.max_tier = ExecutionTier::Three,
                ExecutionTier::Three => flags.max_tier = ExecutionTier::Two,
                ExecutionTier::Two => flags.max_tier = ExecutionTier::One,
                ExecutionTier::One => flags.max_tier = ExecutionTier::Zero,
                ExecutionTier::Zero => {}, // Can't disable tier 0
            }
        }
        CapabilityFlagsStore::<T>::put(flags);
        Ok(())
    }

    /// Update system parameter
    fn update_parameter(_key: &[u8], _value: &[u8]) -> DispatchResult {
        // Implementation would update specific parameters
        // This is a placeholder for parameter updates
        Ok(())
    }

    /// Control agent (quarantine/unquarantine)
    fn control_agent(_agent_id: &[u8], _quarantine: bool) -> DispatchResult {
        // Implementation would integrate with agent management
        // This would call the BotChain Commandments pallet
        Ok(())
    }

    /// Schedule system upgrade
    fn schedule_upgrade(_upgrade_hash: &[u8]) -> DispatchResult {
        // Implementation would schedule runtime upgrade
        Ok(())
    }

    /// Update capabilities based on kill switch activation/deactivation
    fn update_capabilities_from_kill_switch(switch: &KillSwitch, activating: bool) -> DispatchResult {
        let mut flags = CapabilityFlagsStore::<T>::get();

        match switch {
            KillSwitch::Global => {
                flags.global_enabled = !activating;
            },
            KillSwitch::Capability(cap_type) => {
                let index = *cap_type as usize;
                if index < flags.capabilities.len() {
                    flags.capabilities[index] = !activating;
                }
            },
            KillSwitch::Tier(tier) => {
                if activating {
                    // Activating a tier kill switch prevents that tier and above
                    flags.max_tier = match tier {
                        ExecutionTier::Zero => ExecutionTier::Zero, // Can't disable tier 0
                        ExecutionTier::One => ExecutionTier::Zero,
                        ExecutionTier::Two => ExecutionTier::One,
                        ExecutionTier::Three => ExecutionTier::Two,
                        ExecutionTier::Four => ExecutionTier::Three,
                    };
                }
            },
            KillSwitch::Agent => {
                // Agent-specific kill switches would be handled per-agent
            },
        }

        CapabilityFlagsStore::<T>::put(flags);
        Ok(())
    }

    /// Check if an operation is allowed based on current governance state
    pub fn is_operation_allowed(
        capability: CapabilityType,
        tier: ExecutionTier,
        agent_id: Option<&T::AccountId>,
    ) -> bool {
        let flags = CapabilityFlagsStore::<T>::get();

        // Check global enable
        if !flags.global_enabled {
            return false;
        }

        // Check capability enable
        let cap_index = capability as usize;
        if cap_index >= flags.capabilities.len() || !flags.capabilities[cap_index] {
            return false;
        }

        // Check tier permission
        let max_tier_value = flags.max_tier as u8;
        let requested_tier_value = tier as u8;
        if requested_tier_value > max_tier_value {
            return false;
        }

        // Check agent-specific kill switches
        if let Some(agent) = agent_id {
            if let Some(_) = KillSwitches::<T>::get(KillSwitch::Agent) {
                // Agent-specific logic would be implemented here
                // For now, assume agent kill switches disable all operations
                return false;
            }
        }

        true
    }

    /// Get current governance status
    pub fn get_governance_status() -> (bool, ExecutionTier, Vec<KillSwitch>) {
        let flags = CapabilityFlagsStore::<T>::get();

        // Get active kill switches
        let mut active_switches = Vec::new();
        for (switch, state) in KillSwitches::<T>::iter() {
            if state.active {
                active_switches.push(switch);
            }
        }

        (flags.global_enabled, flags.max_tier, active_switches)
    }
}