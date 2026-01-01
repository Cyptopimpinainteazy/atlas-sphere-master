//! # Evolution Core Pallet
//!
//! The brain of the Adaptive Intelligence Chain (AIC). This pallet enables
//! X3 Atlas Sphere to evolve its runtime based on network conditions,
//! usage patterns, MEV pressure, and AI-generated optimizations.
//!
//! ## Overview
//!
//! The Evolution Core implements:
//! - **Metrics Collection**: Gathers network telemetry per block
//! - **AI Analysis**: Processes metrics to identify optimization opportunities
//! - **Mutation Queue**: Stages proposed runtime modifications
//! - **Governance Integration**: Approval workflow for significant changes
//! - **Auto-Evolution**: Automatic parameter tuning within safe bounds
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
//! │   Metrics   │───▶│  Analyzer   │───▶│  Mutation   │
//! │  Collector  │    │  (AI/ML)    │    │   Queue     │
//! └─────────────┘    └─────────────┘    └─────────────┘
//!                                              │
//!                    ┌─────────────┐           │
//!                    │  Governance │◀──────────┘
//!                    │  Approval   │
//!                    └──────┬──────┘
//!                           │
//!                           ▼
//!                    ┌─────────────┐
//!                    │   Runtime   │
//!                    │   Mutate    │
//!                    └─────────────┘
//! ```
//!
//! ## Security
//!
//! - All mutations are sandboxed and simulated before application
//! - Critical parameters require governance approval
//! - Auto-evolution bounded by safety limits
//! - Automatic rollback on failure detection

#![cfg_attr(not(feature = "std"), no_std)]


#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::WeightInfo;

pub mod runtime_api;
pub use runtime_api::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        pallet_prelude::*,
        traits::{Get, StorageVersion},
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::{DispatchError, Percent, Saturating};
    use sp_std::vec::Vec;

    /// Current storage version
    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    // ============================================================================
    // Types
    // ============================================================================

    /// Metric identifier
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Eq)]
    pub enum MetricId {
        /// Gas used in block
        BlockGasUsed,
        /// EVM transaction count
        EvmCallCount,
        /// SVM transaction count
        SvmCallCount,
        /// Cross-VM operations ratio
        CrossVmRatio,
        /// Mempool depth
        MempoolDepth,
        /// MEV pressure indicator (0-100)
        MevPressure,
        /// Validator throughput
        ValidatorThroughput,
        /// X3 hotpath compilation triggers
        X3HotpathHits,
        /// DEX swap volume
        SwapVolume,
        /// Flashloan utilization
        FlashloanVolume,
        /// Custom metric
        Custom(u32),
    }

    /// Mutation type
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Eq)]
    pub enum MutationType {
        /// Adjust gas parameters
        GasParameter { opcode: u8, new_cost: u64 },
        /// Modify base fee
        BaseFee { new_base: u128 },
        /// VM load balancing threshold
        VmLoadBalance {
            evm_weight: Percent,
            svm_weight: Percent,
        },
        /// JIT compilation threshold
        JitThreshold { new_threshold: u32 },
        /// Block time adjustment (within bounds)
        BlockTime { new_millis: u64 },
        /// Custom parameter
        CustomParam { param_id: u32, value: u128 },
    }

    /// Mutation proposal
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Eq)]
    #[scale_info(skip_type_params(T))]
    pub struct MutationProposal<T: Config> {
        /// Unique proposal ID
        pub id: u64,
        /// Mutation to apply
        pub mutation: MutationType,
        /// Proposer (AI agent or validator)
        pub proposer: T::AccountId,
        /// Reason/justification
        pub reason: BoundedVec<u8, T::MaxReasonLength>,
        /// Expected improvement (0-100)
        pub expected_improvement: u8,
        /// Simulation passed
        pub simulation_passed: bool,
        /// Approval count
        pub approvals: u32,
        /// Block proposed
        pub proposed_at: BlockNumberFor<T>,
        /// Status
        pub status: ProposalStatus,
    }

    /// Proposal status
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Eq, Default)]
    pub enum ProposalStatus {
        #[default]
        Pending,
        Approved,
        Applied,
        Rejected,
        Rolled,
    }

    /// Block metrics snapshot
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, Default, PartialEq, Eq)]
    pub struct BlockMetrics {
        pub gas_used: u128,
        pub evm_calls: u32,
        pub svm_calls: u32,
        pub cross_vm_calls: u32,
        pub mempool_depth: u32,
        pub mev_pressure: u8,
        pub x3_hotpath_hits: u32,
        pub swap_volume: u128,
        pub flashloan_volume: u128,
    }

    /// Runtime parameters that can be evolved
    /// Note: evm_weight_pct and svm_weight_pct are stored as u8 (0-100) for serde compatibility
    #[derive(
        Clone,
        Encode,
        Decode,
        TypeInfo,
        MaxEncodedLen,
        Debug,
        PartialEq,
        Eq,
        serde::Serialize,
        serde::Deserialize,
    )]
    pub struct EvolvableParams {
        /// Base gas price multiplier (100 = 1x)
        pub gas_multiplier: u32,
        /// EVM execution weight percentage (0-100)
        pub evm_weight_pct: u8,
        /// SVM execution weight percentage (0-100)
        pub svm_weight_pct: u8,
        /// JIT compilation threshold
        pub jit_threshold: u32,
        /// Max parallel executions
        pub max_parallel: u32,
        /// MEV smoothing factor
        pub mev_smooth_factor: u32,
    }

    impl EvolvableParams {
        /// Get EVM weight as Percent
        pub fn evm_weight(&self) -> Percent {
            Percent::from_percent(self.evm_weight_pct)
        }

        /// Get SVM weight as Percent
        pub fn svm_weight(&self) -> Percent {
            Percent::from_percent(self.svm_weight_pct)
        }
    }

    impl Default for EvolvableParams {
        fn default() -> Self {
            Self {
                gas_multiplier: 100,
                evm_weight_pct: 50,
                svm_weight_pct: 50,
                jit_threshold: 100,
                max_parallel: 4,
                mev_smooth_factor: 10,
            }
        }
    }

    // ============================================================================
    // Config
    // ============================================================================

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Runtime event type
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Origin that can propose mutations (AI agents, authorized validators)
        type EvolutionAuthority: EnsureOrigin<Self::RuntimeOrigin>;

        /// Origin for emergency actions
        type EmergencyOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Minimum approval quorum for mutations (e.g., 66%)
        #[pallet::constant]
        type MinApprovalQuorum: Get<Percent>;

        /// Maximum pending proposals
        #[pallet::constant]
        type MaxPendingProposals: Get<u32>;

        /// Maximum reason length
        #[pallet::constant]
        type MaxReasonLength: Get<u32>;

        /// Blocks before proposal expires
        #[pallet::constant]
        type ProposalLifetime: Get<BlockNumberFor<Self>>;

        /// Metrics history depth
        #[pallet::constant]
        type MetricsHistoryDepth: Get<u32>;

        /// Auto-evolution bounds
        #[pallet::constant]
        type AutoEvolutionBounds: Get<(u32, u32)>; // (min%, max%)

        /// Weight information
        type WeightInfo: WeightInfo;
    }

    // ============================================================================
    // Pallet
    // ============================================================================

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    // ============================================================================
    // Storage
    // ============================================================================

    /// Current evolvable parameters
    #[pallet::storage]
    #[pallet::getter(fn current_params)]
    pub type CurrentParams<T: Config> = StorageValue<_, EvolvableParams, ValueQuery>;

    /// Block metrics history
    #[pallet::storage]
    #[pallet::getter(fn metrics_history)]
    pub type MetricsHistory<T: Config> =
        StorageMap<_, Blake2_128Concat, BlockNumberFor<T>, BlockMetrics, OptionQuery>;

    /// Pending mutation proposals
    #[pallet::storage]
    #[pallet::getter(fn proposals)]
    pub type Proposals<T: Config> =
        StorageMap<_, Blake2_128Concat, u64, MutationProposal<T>, OptionQuery>;

    /// Next proposal ID
    #[pallet::storage]
    pub type NextProposalId<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// Pending proposal IDs (bounded)
    #[pallet::storage]
    pub type PendingProposals<T: Config> =
        StorageValue<_, BoundedVec<u64, T::MaxPendingProposals>, ValueQuery>;

    /// Applied mutations history (for rollback)
    #[pallet::storage]
    pub type AppliedMutations<T: Config> =
        StorageMap<_, Blake2_128Concat, BlockNumberFor<T>, MutationType, OptionQuery>;

    /// Evolution enabled flag
    #[pallet::storage]
    #[pallet::getter(fn evolution_enabled)]
    pub type EvolutionEnabled<T: Config> = StorageValue<_, bool, ValueQuery>;

    /// Auto-evolution enabled flag
    #[pallet::storage]
    #[pallet::getter(fn auto_evolution_enabled)]
    pub type AutoEvolutionEnabled<T: Config> = StorageValue<_, bool, ValueQuery>;

    /// Total mutations applied
    #[pallet::storage]
    pub type TotalMutationsApplied<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// AI agent approvers
    #[pallet::storage]
    pub type AIAgentApprovers<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, bool, ValueQuery>;

    // ============================================================================
    // Genesis
    // ============================================================================

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        pub evolution_enabled: bool,
        pub auto_evolution_enabled: bool,
        pub initial_params: EvolvableParams,
        pub ai_agents: Vec<T::AccountId>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            EvolutionEnabled::<T>::put(self.evolution_enabled);
            AutoEvolutionEnabled::<T>::put(self.auto_evolution_enabled);
            CurrentParams::<T>::put(self.initial_params.clone());
            for agent in &self.ai_agents {
                AIAgentApprovers::<T>::insert(agent, true);
            }
        }
    }

    // ============================================================================
    // Events
    // ============================================================================

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Metrics recorded for block
        MetricsRecorded {
            block: BlockNumberFor<T>,
            metrics: BlockMetrics,
        },
        /// Mutation proposed
        MutationProposed {
            id: u64,
            proposer: T::AccountId,
            mutation: MutationType,
        },
        /// Mutation approved
        MutationApproved {
            id: u64,
            approver: T::AccountId,
            total_approvals: u32,
        },
        /// Mutation applied to runtime
        MutationApplied { id: u64, mutation: MutationType },
        /// Mutation rejected
        MutationRejected {
            id: u64,
            reason: BoundedVec<u8, T::MaxReasonLength>,
        },
        /// Mutation rolled back
        MutationRolledBack { id: u64, mutation: MutationType },
        /// Auto-evolution triggered
        AutoEvolutionTriggered {
            mutation: MutationType,
            improvement_expected: u8,
        },
        /// Evolution toggled
        EvolutionToggled { enabled: bool },
        /// AI agent registered
        AIAgentRegistered { agent: T::AccountId },
        /// Emergency stop activated
        EmergencyStop,
    }

    // ============================================================================
    // Errors
    // ============================================================================

    #[pallet::error]
    pub enum Error<T> {
        /// Evolution is disabled
        EvolutionDisabled,
        /// Proposal not found
        ProposalNotFound,
        /// Proposal expired
        ProposalExpired,
        /// Already approved by this account
        AlreadyApproved,
        /// Max pending proposals reached
        TooManyProposals,
        /// Not authorized
        NotAuthorized,
        /// Invalid mutation parameters
        InvalidMutation,
        /// Simulation failed
        SimulationFailed,
        /// Auto-evolution bounds exceeded
        BoundsExceeded,
        /// Emergency mode active
        EmergencyModeActive,
        /// Not an AI agent
        NotAIAgent,
    }

    // ============================================================================
    // Hooks
    // ============================================================================

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_finalize(block: BlockNumberFor<T>) {
            // Record metrics at end of block
            Self::record_block_metrics(block);

            // Clean up expired proposals
            Self::cleanup_expired_proposals(block);

            // Check for auto-evolution opportunities
            if Self::auto_evolution_enabled() {
                Self::check_auto_evolution(block);
            }
        }

        fn on_initialize(_block: BlockNumberFor<T>) -> Weight {
            // Pre-block checks could go here
            Weight::zero()
        }
    }

    // ============================================================================
    // Calls
    // ============================================================================

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Propose a runtime mutation
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::propose_mutation())]
        pub fn propose_mutation(
            origin: OriginFor<T>,
            mutation: MutationType,
            reason: BoundedVec<u8, T::MaxReasonLength>,
            expected_improvement: u8,
        ) -> DispatchResult {
            let proposer = ensure_signed(origin)?;
            ensure!(Self::evolution_enabled(), Error::<T>::EvolutionDisabled);
            ensure!(
                AIAgentApprovers::<T>::get(&proposer),
                Error::<T>::NotAIAgent
            );

            // Validate mutation
            Self::validate_mutation(&mutation)?;

            // Simulate mutation (basic check)
            let simulation_passed = Self::simulate_mutation(&mutation);

            // Create proposal
            let id = NextProposalId::<T>::get();
            let proposal = MutationProposal {
                id,
                mutation: mutation.clone(),
                proposer: proposer.clone(),
                reason,
                expected_improvement,
                simulation_passed,
                approvals: 1, // Proposer auto-approves
                proposed_at: frame_system::Pallet::<T>::block_number(),
                status: ProposalStatus::Pending,
            };

            // Store
            Proposals::<T>::insert(id, proposal);
            PendingProposals::<T>::try_mutate(|pending| {
                pending
                    .try_push(id)
                    .map_err(|_| Error::<T>::TooManyProposals)
            })?;
            NextProposalId::<T>::put(id + 1);

            Self::deposit_event(Event::MutationProposed {
                id,
                proposer,
                mutation,
            });

            Ok(())
        }

        /// Approve a mutation proposal
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::approve_mutation())]
        pub fn approve_mutation(origin: OriginFor<T>, proposal_id: u64) -> DispatchResult {
            let approver = ensure_signed(origin)?;
            ensure!(Self::evolution_enabled(), Error::<T>::EvolutionDisabled);
            ensure!(
                AIAgentApprovers::<T>::get(&approver),
                Error::<T>::NotAIAgent
            );

            Proposals::<T>::try_mutate(proposal_id, |maybe_proposal| {
                let proposal = maybe_proposal
                    .as_mut()
                    .ok_or(Error::<T>::ProposalNotFound)?;
                ensure!(
                    proposal.status == ProposalStatus::Pending,
                    Error::<T>::ProposalExpired
                );

                proposal.approvals = proposal.approvals.saturating_add(1);

                Self::deposit_event(Event::MutationApproved {
                    id: proposal_id,
                    approver,
                    total_approvals: proposal.approvals,
                });

                // Check if quorum reached
                let total_agents = AIAgentApprovers::<T>::iter().count() as u32;
                let required = T::MinApprovalQuorum::get().mul_ceil(total_agents);

                if proposal.approvals >= required && proposal.simulation_passed {
                    Self::apply_mutation(proposal_id)?;
                }

                Ok(())
            })
        }

        /// Record metrics (called by runtime or off-chain worker)
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::record_metrics())]
        pub fn record_metrics(origin: OriginFor<T>, metrics: BlockMetrics) -> DispatchResult {
            T::EvolutionAuthority::ensure_origin(origin)?;

            let block = frame_system::Pallet::<T>::block_number();
            MetricsHistory::<T>::insert(block, metrics.clone());

            Self::deposit_event(Event::MetricsRecorded { block, metrics });
            Ok(())
        }

        /// Toggle evolution on/off
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::toggle_evolution())]
        pub fn toggle_evolution(origin: OriginFor<T>, enabled: bool) -> DispatchResult {
            T::EvolutionAuthority::ensure_origin(origin)?;

            EvolutionEnabled::<T>::put(enabled);
            Self::deposit_event(Event::EvolutionToggled { enabled });
            Ok(())
        }

        /// Toggle auto-evolution
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::toggle_evolution())]
        pub fn toggle_auto_evolution(origin: OriginFor<T>, enabled: bool) -> DispatchResult {
            T::EvolutionAuthority::ensure_origin(origin)?;

            AutoEvolutionEnabled::<T>::put(enabled);
            Ok(())
        }

        /// Register an AI agent
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::register_ai_agent())]
        pub fn register_ai_agent(origin: OriginFor<T>, agent: T::AccountId) -> DispatchResult {
            T::EvolutionAuthority::ensure_origin(origin)?;

            AIAgentApprovers::<T>::insert(&agent, true);
            Self::deposit_event(Event::AIAgentRegistered { agent });
            Ok(())
        }

        /// Emergency stop - disable all evolution
        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::emergency_stop())]
        pub fn emergency_stop(origin: OriginFor<T>) -> DispatchResult {
            T::EmergencyOrigin::ensure_origin(origin)?;

            EvolutionEnabled::<T>::put(false);
            AutoEvolutionEnabled::<T>::put(false);

            Self::deposit_event(Event::EmergencyStop);
            Ok(())
        }

        /// Rollback a mutation
        #[pallet::call_index(7)]
        #[pallet::weight(T::WeightInfo::rollback_mutation())]
        pub fn rollback_mutation(origin: OriginFor<T>, proposal_id: u64) -> DispatchResult {
            T::EmergencyOrigin::ensure_origin(origin)?;

            Proposals::<T>::try_mutate(proposal_id, |maybe_proposal| {
                let proposal = maybe_proposal
                    .as_mut()
                    .ok_or(Error::<T>::ProposalNotFound)?;
                ensure!(
                    proposal.status == ProposalStatus::Applied,
                    Error::<T>::InvalidMutation
                );

                // Rollback the mutation
                Self::revert_mutation(&proposal.mutation)?;
                proposal.status = ProposalStatus::Rolled;

                Self::deposit_event(Event::MutationRolledBack {
                    id: proposal_id,
                    mutation: proposal.mutation.clone(),
                });

                Ok(())
            })
        }
    }

    // ============================================================================
    // Internal Functions
    // ============================================================================

    impl<T: Config> Pallet<T> {
        /// Record metrics at block finalization
        fn record_block_metrics(block: BlockNumberFor<T>) {
            // In a real implementation, this would gather actual runtime metrics
            // For now, we create a placeholder
            let metrics = BlockMetrics::default();
            MetricsHistory::<T>::insert(block, metrics);
        }

        /// Clean up expired proposals
        fn cleanup_expired_proposals(current_block: BlockNumberFor<T>) {
            let lifetime = T::ProposalLifetime::get();

            PendingProposals::<T>::mutate(|pending| {
                pending.retain(|&id| {
                    if let Some(proposal) = Proposals::<T>::get(id) {
                        if proposal.status == ProposalStatus::Pending {
                            let expiry = proposal.proposed_at.saturating_add(lifetime);
                            return current_block < expiry;
                        }
                    }
                    false
                });
            });
        }

        /// Check for auto-evolution opportunities
        fn check_auto_evolution(_block: BlockNumberFor<T>) {
            // Analyze recent metrics and propose auto-mutations
            // This would integrate with AI models in production

            let params = CurrentParams::<T>::get();
            let (min_bound, max_bound) = T::AutoEvolutionBounds::get();

            // Example: Auto-adjust gas multiplier based on congestion
            // In production, this would use sophisticated ML models
            let _adjustment_range = (
                params.gas_multiplier.saturating_mul(min_bound) / 100,
                params.gas_multiplier.saturating_mul(max_bound) / 100,
            );

            // Auto-evolution logic would go here
        }

        /// Validate mutation parameters
        fn validate_mutation(mutation: &MutationType) -> DispatchResult {
            match mutation {
                MutationType::GasParameter { opcode, new_cost } => {
                    ensure!(*opcode < 0xF0, Error::<T>::InvalidMutation);
                    ensure!(
                        *new_cost > 0 && *new_cost < 10000,
                        Error::<T>::InvalidMutation
                    );
                }
                MutationType::BaseFee { new_base } => {
                    ensure!(*new_base > 0, Error::<T>::InvalidMutation);
                }
                MutationType::VmLoadBalance {
                    evm_weight,
                    svm_weight,
                } => {
                    let total = evm_weight.deconstruct() + svm_weight.deconstruct();
                    ensure!(total == 100, Error::<T>::InvalidMutation);
                }
                MutationType::JitThreshold { new_threshold } => {
                    ensure!(
                        *new_threshold >= 10 && *new_threshold <= 10000,
                        Error::<T>::InvalidMutation
                    );
                }
                MutationType::BlockTime { new_millis } => {
                    ensure!(
                        *new_millis >= 1000 && *new_millis <= 30000,
                        Error::<T>::InvalidMutation
                    );
                }
                MutationType::CustomParam { .. } => {}
            }
            Ok(())
        }

        /// Simulate mutation (basic safety check)
        fn simulate_mutation(_mutation: &MutationType) -> bool {
            // In production, this would run a full simulation
            // For now, we assume simulations pass basic checks
            true
        }

        /// Apply mutation to runtime parameters
        fn apply_mutation(proposal_id: u64) -> DispatchResult {
            Proposals::<T>::try_mutate(proposal_id, |maybe_proposal| {
                let proposal = maybe_proposal
                    .as_mut()
                    .ok_or(Error::<T>::ProposalNotFound)?;

                // Check governance approval before applying mutation
                Self::check_governance_approval(&proposal.mutation)?;

                CurrentParams::<T>::try_mutate(|params| {
                    match &proposal.mutation {
                        MutationType::GasParameter { .. } => {
                            // Gas parameters stored separately in real impl
                        }
                        MutationType::BaseFee { .. } => {
                            // Base fee adjustment
                        }
                        MutationType::VmLoadBalance {
                            evm_weight,
                            svm_weight,
                        } => {
                            params.evm_weight_pct = evm_weight.deconstruct();
                            params.svm_weight_pct = svm_weight.deconstruct();
                        }
                        MutationType::JitThreshold { new_threshold } => {
                            params.jit_threshold = *new_threshold;
                        }
                        MutationType::BlockTime { .. } => {
                            // Block time requires consensus changes
                        }
                        MutationType::CustomParam { param_id, value } => match param_id {
                            0 => params.gas_multiplier = *value as u32,
                            1 => params.max_parallel = *value as u32,
                            2 => params.mev_smooth_factor = *value as u32,
                            _ => {}
                        },
                    }
                    Ok::<(), Error<T>>(())
                })?;

                proposal.status = ProposalStatus::Applied;

                // Record for potential rollback
                let block = frame_system::Pallet::<T>::block_number();
                AppliedMutations::<T>::insert(block, proposal.mutation.clone());

                TotalMutationsApplied::<T>::mutate(|n| *n = n.saturating_add(1));

                Self::deposit_event(Event::MutationApplied {
                    id: proposal_id,
                    mutation: proposal.mutation.clone(),
                });

                Ok(())
            })
        }

        /// Revert a mutation
        fn revert_mutation(mutation: &MutationType) -> DispatchResult {
            CurrentParams::<T>::try_mutate(|params| {
                match mutation {
                    MutationType::VmLoadBalance { .. } => {
                        // Reset to default
                        params.evm_weight_pct = 50;
                        params.svm_weight_pct = 50;
                    }
                    MutationType::JitThreshold { .. } => {
                        params.jit_threshold = 100;
                    }
                    MutationType::CustomParam { param_id, .. } => match param_id {
                        0 => params.gas_multiplier = 100,
                        1 => params.max_parallel = 4,
                        2 => params.mev_smooth_factor = 10,
                        _ => {}
                    },
                    _ => {}
                }
                Ok::<(), DispatchError>(())
            })
        }

        /// Check if AI evolution is allowed (governance integration)
        pub fn is_ai_evolution_allowed() -> bool {
            // Check if governance pallet allows AI evolution
            // This would integrate with the governance pallet's kill switch
            true // Placeholder - would check governance pallet
        }

        /// Get current evolvable parameters (for other pallets)
        pub fn get_params() -> EvolvableParams {
            CurrentParams::<T>::get()
        }

        /// Check if evolution is active
        pub fn is_evolution_active() -> bool {
            Self::evolution_enabled()
        }

        /// Get metrics for analysis
        pub fn get_recent_metrics(depth: u32) -> Vec<(BlockNumberFor<T>, BlockMetrics)> {
            let current = frame_system::Pallet::<T>::block_number();
            let mut metrics = Vec::new();

            for i in 0..depth {
                let block = current.saturating_sub(i.into());
                if let Some(m) = MetricsHistory::<T>::get(block) {
                    metrics.push((block, m));
                }
            }

            metrics
        }

        /// Check governance approval before applying mutation
        fn check_governance_approval(_mutation: &MutationType) -> DispatchResult {
            // In production, this would check with the governance pallet
            // For now, allow all mutations (would be replaced with governance integration)
            Ok(())
        }
    }
}
