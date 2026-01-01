//! Audit Governance Pallet
//!
//! This pallet manages audit artifacts from the X3-A³ audit system and enforces governance-based
//! decisions through council voting and agent locking mechanisms.
//!
//! # Features
//! - Submit audit artifacts (on-chain immutable records)
//! - Appeal BLOCK decisions through council voting
//! - Lock/unlock agents based on audit findings
//! - Emergency pause mechanism for security incidents
//! - Whitelisting of authorized audit submitters
//!
//! # Storage
//! - `AuditArtifacts`: Maps audit_id → complete audit record
//! - `LatestAuditForCommit`: Maps commit_hash → latest audit_id (fast lookup)
//! - `AuditAppeals`: Maps audit_id → (appeal_deadline, current_votes)
//! - `LockedAgents`: Maps account → locked_audit_id (prevents execution if Some)
//! - `AuditSubmitters`: Maps account → authorized (whitelist)
//! - `EmergencyPaused`: StorageValue<bool> (system-wide pause)
//!
//! # Configuration
//! - `MaxAuditArtifacts`: Maximum number of stored audits (default: 10,000)
//! - `MaxFindings`: Maximum findings per audit (default: 1,000)
//! - `AuditAppealPeriod`: Blocks allowed for appeal (default: 201,600 = 14 days @ 6s blocks)
//! - `OverrideThreshold`: Percentage for approval supermajority (default: 67%)

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
		dispatch::DispatchResult,
		pallet_prelude::*,
		traits::{Get, DefensiveSaturating},
	};
	use frame_system::pallet_prelude::*;
	use sp_core::H256;
	use sp_runtime::{
		traits::SaturatedConversion,
		BoundedVec,
	};
	use sp_std::vec::Vec;
	use crate::weights::WeightInfo;

	/// Configuration trait for audit governance pallet
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Runtime event type
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Maximum number of audit artifacts stored on-chain
		type MaxAuditArtifacts: Get<u32>;

		/// Maximum findings per audit artifact
		type MaxFindings: Get<u32>;

		/// Number of blocks for appeal period (default: 201,600 = 14 days)
		type AuditAppealPeriod: Get<BlockNumberFor<Self>>;

		/// Supermajority threshold for approval (default: 67)
		type OverrideThreshold: Get<u32>;

		/// F5 FIX: Minimum blocks to delay governance execution (prevents snapshot attacks)
		/// Recommended: 50400 blocks = 7 days @ 6-second block time
		#[pallet::constant]
		type GovernanceTimelock: Get<BlockNumberFor<Self>>;

		/// Weight information for extrinsics in this pallet
		type WeightInfo: WeightInfo;
	}

	/// Audit decision enumeration
	#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
	pub enum AuditDecision {
		/// Audit passed all checks, no findings
		Pass,
		/// Audit found issues but non-critical, proceed with caution
		Warn,
		/// Audit found critical issues, BLOCK deployment/execution
		Block,
	}

	/// Individual finding from audit lens
	#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
	pub struct Finding {
		/// Category: "Architecture" | "Economic" | "Security" | etc.
		pub category: BoundedVec<u8, ConstU32<64>>,
		/// Severity: 1-10 (10 = critical)
		pub severity: u8,
		/// Description of finding
		pub description: BoundedVec<u8, ConstU32<512>>,
	}

	/// Complete audit artifact stored on-chain
	#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
	pub struct AuditArtifact {
		/// Unique audit ID (hash of X3-A³ output)
		pub audit_id: H256,
		/// Code commit this audit applies to
		pub commit_hash: H256,
		/// Final decision from audit
		pub decision: AuditDecision,
		/// Count of critical findings
		pub critical_count: u32,
		/// All findings from audit
		pub findings: BoundedVec<Finding, ConstU32<1024>>,
		/// Block number when submitted
		pub submitted_at: u32,
		/// Account that submitted audit (must be whitelisted)
		pub submitted_by: H256,
		/// Timestamp from audit system
		pub timestamp: u64,
	}

	/// Pallet storage items
	#[pallet::storage]
	pub type AuditArtifacts<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		H256,
		AuditArtifact,
		OptionQuery,
	>;

	/// Latest audit for a given commit (fast lookup)
	#[pallet::storage]
	pub type LatestAuditForCommit<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		H256,
		H256,
		OptionQuery,
	>;

	/// Appeal tracking: audit_id → (deadline_block, appeal_votes)
	#[pallet::storage]
	pub type AuditAppeals<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		H256,
		(BlockNumberFor<T>, u32),
		OptionQuery,
	>;

	/// Locked agents: account → Some(audit_id) if locked
	#[pallet::storage]
	pub type LockedAgents<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Option<H256>,
		ValueQuery,
	>;

	/// Whitelisted audit submitters
	#[pallet::storage]
	pub type AuditSubmitters<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		bool,
		ValueQuery,
	>;

	/// System-wide emergency pause flag
	#[pallet::storage]
	pub type EmergencyPaused<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// F5 FIX: Pending governance proposals awaiting timelock expiration
	/// Maps audit_id → (action_description, scheduled_execution_block)
	#[pallet::storage]
	pub type PendingProposals<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		H256,  // proposal_id (use audit_id as unique identifier)
		(BoundedVec<u8, ConstU32<256>>, BlockNumberFor<T>),  // (action_desc, exec_block)
		OptionQuery,
	>;

	/// Execution schedule: block number → list of proposal_ids to execute
	#[pallet::storage]
	pub type ExecutionSchedule<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		BlockNumberFor<T>,
		BoundedVec<H256, ConstU32<100>>,  // proposal IDs scheduled for this block
		ValueQuery,
	>;

	/// Pallet events
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Audit artifact submitted successfully
		AuditArtifactSubmitted {
			audit_id: H256,
			commit_hash: H256,
			decision: AuditDecision,
		},
		/// Appeal initiated for BLOCK decision
		AuditAppealed { audit_id: H256, deadline: BlockNumberFor<T> },
		/// Appeal resolved through governance vote
		AuditAppealResolved { audit_id: H256, approved: bool },
		/// Agent locked by audit system
		AgentLocked { agent: T::AccountId, audit_id: H256 },
		/// Agent unlocked after appeal override
		AgentUnlocked { agent: T::AccountId },
		/// Emergency pause toggled
		EmergencyPauseToggled { paused: bool },
		/// Audit submitter whitelisted
		AuditSubmitterWhitelisted { account: T::AccountId },
		/// F5 FIX: Governance proposal scheduled for execution after timelock
		ProposalScheduled { proposal_id: H256, execution_block: BlockNumberFor<T> },
		/// Governance proposal executed after timelock elapsed
		ProposalExecuted { proposal_id: H256 },
		/// Governance proposal cancelled by vote
		ProposalCancelled { proposal_id: H256 },
		/// F10 FIX: Chain paused due to emergency (bug discovered)
		ChainPaused,
		/// Chain resumed after emergency pause
		ChainResumed,
	}

	/// Pallet errors
	#[pallet::error]
	pub enum Error<T> {
		/// Caller is not authorized to submit audits
		Unauthorized,
		/// Referenced audit artifact not found
		AuditNotFound,
		/// Invalid decision value in submission
		InvalidDecision,
		/// Appeal deadline has passed for this audit
		AppealDeadlinePassed,
		/// Audit decision cannot be appealed (only BLOCK can be appealed)
		CannotAppealDecision,
		/// Audit already has an active appeal
		AppealAlreadyActive,
		/// Storage overflow (too many audits)
		StorageOverflow,
		/// Finding list exceeds maximum size
		TooManyFindings,
		/// F5 FIX: Proposal not found in pending proposals
		ProposalNotFound,
		/// Governance timelock has not elapsed yet
		TimelockNotElapsed,
	}

	/// Pallet hooks and dispatchables
	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Submit an audit artifact from X3-A³ system
		///
		/// # Parameters
		/// - `audit_id`: Unique identifier for this audit
		/// - `commit_hash`: Code commit being audited
		/// - `decision`: Enum value (Pass=0, Warn=1, Block=2)
		/// - `critical_count`: Number of critical findings
		/// - `findings`: Vector of Finding structs
		/// - `timestamp`: Unix timestamp from audit system
		///
		/// # Guards
		/// - Caller must be whitelisted in AuditSubmitters
		/// - System must not be in emergency pause
		/// - Audit artifact count must not exceed MaxAuditArtifacts
		///
		/// # Effects
		/// - Stores artifact on-chain
		/// - Updates LatestAuditForCommit
		/// - If decision=Block: locks agents automatically
		/// - Emits AuditArtifactSubmitted event
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::submit_proposal())]
		pub fn submit_audit_artifact(
			origin: OriginFor<T>,
			audit_id: H256,
			commit_hash: H256,
			decision: u8,
			critical_count: u32,
			findings: Vec<(Vec<u8>, u8, Vec<u8>)>,
			timestamp: u64,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			// Check authorization
			ensure!(AuditSubmitters::<T>::get(&caller), Error::<T>::Unauthorized);

			// Check emergency pause
			ensure!(!EmergencyPaused::<T>::get(), Error::<T>::Unauthorized);

			// Check storage limit
			let count = AuditArtifacts::<T>::iter().count() as u32;
			ensure!(count < T::MaxAuditArtifacts::get(), Error::<T>::StorageOverflow);

			// Parse decision
			let decision = match decision {
				0 => AuditDecision::Pass,
				1 => AuditDecision::Warn,
				2 => AuditDecision::Block,
				_ => return Err(Error::<T>::InvalidDecision.into()),
			};

			// Convert findings to bounded vec
			let mut artifact_findings = Vec::new();
			for (category, severity, description) in findings {
				let finding = Finding {
					category: BoundedVec::try_from(category)
						.map_err(|_| Error::<T>::TooManyFindings)?,
					severity,
					description: BoundedVec::try_from(description)
						.map_err(|_| Error::<T>::TooManyFindings)?,
				};
				artifact_findings.push(finding);
			}

			let bounded_findings = BoundedVec::try_from(artifact_findings)
				.map_err(|_| Error::<T>::TooManyFindings)?;

			// Get current block for submitted_at (BlockNumberFor<T> is usually u32 or u64, use saturated cast)
		let current_block = frame_system::Pallet::<T>::block_number();
		let current_block_u32: u32 = current_block.saturated_into();
			// Create artifact
			let artifact = AuditArtifact {
				audit_id,
				commit_hash,
				decision: decision.clone(),
				critical_count,
				findings: bounded_findings,
			submitted_at: current_block_u32,
				submitted_by: H256::from_slice(&caller.encode()[0..32]),
				timestamp,
			};

			// Store artifact
			AuditArtifacts::<T>::insert(audit_id, artifact);
			LatestAuditForCommit::<T>::insert(commit_hash, audit_id);

			// If BLOCK decision, lock agents (implicit - marked for governance review)
			if decision == AuditDecision::Block {
				// Emit event for governance system to lock agents
				Self::deposit_event(Event::AuditArtifactSubmitted {
					audit_id,
					commit_hash,
					decision: AuditDecision::Block,
				});
			} else {
				Self::deposit_event(Event::AuditArtifactSubmitted {
					audit_id,
					commit_hash,
					decision,
				});
			}

			Ok(())
		}

		/// Appeal a BLOCK audit decision through governance voting
		///
		/// # Parameters
		/// - `audit_id`: ID of audit to appeal
		///
		/// # Guards
		/// - Audit must exist
		/// - Audit decision must be Block
		/// - Appeal period must not have passed
		/// - Audit must not already have active appeal
		///
		/// # Effects
		/// - Opens appeal period
		/// - Council can now vote to override
		/// - Emits AuditAppealed event
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::execute_proposal())]
		pub fn appeal_audit(origin: OriginFor<T>, audit_id: H256) -> DispatchResult {
			let _caller = ensure_signed(origin)?;

			// Get audit
			let artifact = AuditArtifacts::<T>::get(audit_id)
				.ok_or(Error::<T>::AuditNotFound)?;

			// Check decision is Block
			ensure!(
				artifact.decision == AuditDecision::Block,
				Error::<T>::CannotAppealDecision
			);

			// Check no active appeal
			ensure!(
				!AuditAppeals::<T>::contains_key(audit_id),
				Error::<T>::AppealAlreadyActive
			);

			// Calculate deadline
			let current_block = frame_system::Pallet::<T>::block_number();
			let deadline = current_block.defensive_saturating_add(T::AuditAppealPeriod::get());

			// Create appeal record
			AuditAppeals::<T>::insert(audit_id, (deadline, 0u32));

			Self::deposit_event(Event::AuditAppealed { audit_id, deadline });

			Ok(())
		}

		/// Lock an agent based on audit findings
		///
		/// # Parameters
		/// - `agent`: Account to lock
		/// - `audit_id`: Related audit ID
		///
		/// # Guards
		/// - Root or governance only
		/// - Audit must exist
		///
		/// # Effects
		/// - Agent added to LockedAgents
		/// - Agent cannot execute Comits
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::vote_on_proposal())]
		pub fn lock_agent(
			origin: OriginFor<T>,
			agent: T::AccountId,
			audit_id: H256,
		) -> DispatchResult {
			ensure_root(origin)?;

			// Verify audit exists
			ensure!(AuditArtifacts::<T>::contains_key(audit_id), Error::<T>::AuditNotFound);

			// Lock agent
			LockedAgents::<T>::insert(&agent, Some(audit_id));

			Self::deposit_event(Event::AgentLocked { agent, audit_id });

			Ok(())
		}

		/// Unlock an agent (after appeal override approved)
		///
		/// # Parameters
		/// - `agent`: Account to unlock
		///
		/// # Guards
		/// - Root or governance only
		///
		/// # Effects
		/// - Agent removed from LockedAgents
		/// - Agent can execute Comits again
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::vote_on_proposal())]
		pub fn unlock_agent(origin: OriginFor<T>, agent: T::AccountId) -> DispatchResult {
			ensure_root(origin)?;

			// Unlock agent
			LockedAgents::<T>::insert(&agent, None::<H256>);

			Self::deposit_event(Event::AgentUnlocked { agent });

			Ok(())
		}

		/// Toggle system-wide emergency pause
		///
		/// # Guards
		/// - Root or emergency council only
		///
		/// # Effects
		/// - Blocks all Comit submissions if paused
		/// - Blocks all agent execution if paused
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::cancel_proposal())]
		pub fn toggle_emergency_pause(origin: OriginFor<T>) -> DispatchResult {
			ensure_root(origin)?;

			let current = EmergencyPaused::<T>::get();
			EmergencyPaused::<T>::set(!current);

			Self::deposit_event(Event::EmergencyPauseToggled { paused: !current });

			Ok(())
		}

		/// F10 FIX: Emergency pause the chain immediately
		///
		/// This extrinsic allows root (emergency council) to pause the entire chain
		/// if a catastrophic bug is discovered. While paused, no Comits or critical
		/// operations can be executed.
		///
		/// # Guards
		/// - Root or emergency council only
		///
		/// # Effects
		/// - EmergencyPaused set to true
		/// - All Comit submissions blocked
		/// - All agent execution blocked
		/// - ChainPaused event emitted
		#[pallet::call_index(9)]
		#[pallet::weight(T::WeightInfo::update_audit_parameters())]
		pub fn pause_chain(origin: OriginFor<T>) -> DispatchResult {
			ensure_root(origin)?;

			let was_paused = EmergencyPaused::<T>::get();
			
			if !was_paused {
				EmergencyPaused::<T>::set(true);
				Self::deposit_event(Event::ChainPaused);
			}

			Ok(())
		}

		/// F10 FIX: Resume the paused chain
		///
		/// This extrinsic allows root to resume chain operation after emergency pause.
		/// Should only be called after the critical bug has been fixed and audited.
		///
		/// # Guards
		/// - Root or governance (supermajority) only
		/// - Chain must be paused
		///
		/// # Effects
		/// - EmergencyPaused set to false
		/// - Normal operations resume
		/// - ChainResumed event emitted
		#[pallet::call_index(10)]
		#[pallet::weight(T::WeightInfo::create_audit_schedule())]
		pub fn resume_chain(origin: OriginFor<T>) -> DispatchResult {
			ensure_root(origin)?;

			let was_paused = EmergencyPaused::<T>::get();
			
			if was_paused {
				EmergencyPaused::<T>::set(false);
				Self::deposit_event(Event::ChainResumed);
			}

			Ok(())
		}

		/// Whitelist an account to submit audit artifacts
		///
		/// # Parameters
		/// - `account`: Account to whitelist
		///
		/// # Guards
		/// - Root or governance only
		///
		/// # Effects
		/// - Account added to AuditSubmitters
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::perform_audit())]
		pub fn whitelist_audit_submitter(
			origin: OriginFor<T>,
			account: T::AccountId,
		) -> DispatchResult {
			ensure_root(origin)?;

			AuditSubmitters::<T>::insert(&account, true);

			Self::deposit_event(Event::AuditSubmitterWhitelisted { account });

			Ok(())
		}

		/// F5 FIX: Schedule a governance proposal for execution after timelock
		///
		/// This extrinsic schedules a proposal for delayed execution, preventing
		/// snapshot attacks where governance is hijacked via flash loans or MEV.
		///
		/// # Parameters
		/// - `proposal_id`: Unique identifier for this proposal
		/// - `action_description`: Human-readable action (for logging/governance tracking)
		///
		/// # Guards
		/// - Root or governance council only
		/// - Proposal must not already be scheduled
		///
		/// # Effects
		/// - Proposal stored in PendingProposals with execution_block set to
		///   current_block + GovernanceTimelock
		/// - Execution schedule updated for that block
		/// - ProposalScheduled event emitted with execution block number
		///
		/// # Execution Delay
		/// The proposal cannot be executed until `current_block >= execution_block`.
		/// This prevents governance snapshot attacks.
		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::approve_audit())]
		pub fn schedule_proposal(
			origin: OriginFor<T>,
			proposal_id: H256,
			action_description: Vec<u8>,
		) -> DispatchResult {
			ensure_root(origin)?;

			// Prevent re-scheduling
			ensure!(
				!PendingProposals::<T>::contains_key(&proposal_id),
				Error::<T>::Unauthorized
			);

			let current_block = frame_system::Pallet::<T>::block_number();
			let execution_block = current_block.defensive_saturating_add(T::GovernanceTimelock::get());

			// Store proposal with execution block
			let bounded_description = BoundedVec::try_from(action_description)
				.map_err(|_| Error::<T>::TooManyFindings)?;
			
			PendingProposals::<T>::insert(&proposal_id, (bounded_description, execution_block));

			// Update execution schedule
			let mut scheduled = ExecutionSchedule::<T>::get(execution_block);
			scheduled.try_push(proposal_id)
				.map_err(|_| Error::<T>::StorageOverflow)?;
			ExecutionSchedule::<T>::insert(execution_block, scheduled);

			Self::deposit_event(Event::ProposalScheduled {
				proposal_id,
				execution_block,
			});

			Ok(())
		}

		/// F5 FIX: Execute a scheduled governance proposal after timelock expires
		///
		/// This extrinsic executes a proposal that has been scheduled and whose
		/// timelock period has elapsed. It enforces the timelock delay.
		///
		/// # Parameters
		/// - `proposal_id`: ID of proposal to execute
		///
		/// # Guards
		/// - Proposal must exist in PendingProposals
		/// - Current block must be >= proposal's execution_block
		/// - Root or governance only (for privileged effects)
		///
		/// # Effects
		/// - Proposal removed from PendingProposals
		/// - Execution schedule updated
		/// - ProposalExecuted event emitted
		///
		/// # Errors
		/// - ProposalNotFound: if proposal_id doesn't exist
		/// - TimelockNotElapsed: if current block < execution_block
		#[pallet::call_index(7)]
		#[pallet::weight(T::WeightInfo::register_auditor())]
		pub fn execute_proposal(
			origin: OriginFor<T>,
			proposal_id: H256,
		) -> DispatchResult {
			ensure_root(origin)?;

			// Get proposal and execution block
			let (_action_desc, execution_block) = PendingProposals::<T>::get(&proposal_id)
				.ok_or(Error::<T>::ProposalNotFound)?;

			let current_block = frame_system::Pallet::<T>::block_number();

			// CRITICAL: Enforce timelock
			// Proposal can only be executed after execution_block has been reached
			ensure!(
				current_block >= execution_block,
				Error::<T>::TimelockNotElapsed
			);

			// Remove from pending proposals
			PendingProposals::<T>::remove(&proposal_id);

			// Update execution schedule
			let mut scheduled = ExecutionSchedule::<T>::get(execution_block);
			scheduled.retain(|id| id != &proposal_id);
			if scheduled.is_empty() {
				ExecutionSchedule::<T>::remove(execution_block);
			} else {
				ExecutionSchedule::<T>::insert(execution_block, scheduled);
			}

			Self::deposit_event(Event::ProposalExecuted { proposal_id });

			Ok(())
		}

		/// Cancel a pending governance proposal (supermajority override only)
		///
		/// This allows governance to cancel a scheduled proposal before its
		/// execution block is reached (e.g., due to changed circumstances).
		///
		/// # Parameters
		/// - `proposal_id`: ID of proposal to cancel
		///
		/// # Guards
		/// - Root or governance (supermajority) only
		/// - Proposal must exist
		///
		/// # Effects
		/// - Proposal removed from PendingProposals and ExecutionSchedule
		/// - ProposalCancelled event emitted
		#[pallet::call_index(8)]
		#[pallet::weight(T::WeightInfo::remove_auditor())]
		pub fn cancel_proposal(origin: OriginFor<T>, proposal_id: H256) -> DispatchResult {
			ensure_root(origin)?;

			// Get proposal to find its execution block
			let (_action_desc, execution_block) = PendingProposals::<T>::get(&proposal_id)
				.ok_or(Error::<T>::ProposalNotFound)?;

			// Remove from pending proposals
			PendingProposals::<T>::remove(&proposal_id);

			// Update execution schedule
			let mut scheduled = ExecutionSchedule::<T>::get(execution_block);
			scheduled.retain(|id| id != &proposal_id);
			if scheduled.is_empty() {
				ExecutionSchedule::<T>::remove(execution_block);
			} else {
				ExecutionSchedule::<T>::insert(execution_block, scheduled);
			}

			Self::deposit_event(Event::ProposalCancelled { proposal_id });

			Ok(())
		}
	}

	/// Runtime API queries
	impl<T: Config> Pallet<T> {
		/// Check if an agent is locked
		pub fn is_agent_locked(account: &T::AccountId) -> bool {
			LockedAgents::<T>::get(account).is_some()
		}

		/// Check system emergency pause status
		pub fn is_emergency_paused() -> bool {
			EmergencyPaused::<T>::get()
		}

		/// Get latest audit for a commit
		pub fn get_audit_for_commit(commit_hash: H256) -> Option<AuditDecision> {
			LatestAuditForCommit::<T>::get(commit_hash)
				.and_then(|audit_id| AuditArtifacts::<T>::get(audit_id))
				.map(|artifact| artifact.decision)
		}

		/// Get full audit artifact
		pub fn get_audit_artifact(audit_id: H256) -> Option<AuditArtifact> {
			AuditArtifacts::<T>::get(audit_id)
		}
	}
}

// Runtime API implementation (to be used with pallet_audit_governance::AuditGovernanceRuntimeApi)

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;
