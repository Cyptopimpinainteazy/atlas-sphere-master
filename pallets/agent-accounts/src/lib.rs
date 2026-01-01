//! # X3Chain Agent Accounts Pallet
//!
//! On-chain identities, permissions, and quotas for autonomous AI agents.
//!
//! ## Overview
//!
//! This pallet provides:
//! - On-chain AI agent registration with unique AgentId
//! - Wallet quotas (gas limits, compute limits per block/epoch)
//! - Granular permissions (deploy contracts, stake, vote, trade)
//! - Event streaming for off-chain watchers
//! - Agent lifecycle management (active, suspended, terminated)
//!
//! ## Agent Model
//!
//! Each agent has:
//! - A controller (human account that manages the agent)
//! - An operator (account that can execute on behalf of agent)
//! - Quota limits for gas and compute per block and epoch
//! - Permission flags for various on-chain operations
//! - Reputation score based on behavior

#![cfg_attr(not(feature = "std"), no_std)]


#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::WeightInfo;

pub mod types;
pub use types::*;

pub mod runtime_api;
pub use runtime_api::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, ReservableCurrency},
        Blake2_128Concat,
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::Saturating;
    use sp_std::prelude::*;

    type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    use frame_support::traits::StorageVersion;

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    #[pallet::pallet]
    #[pallet::without_storage_info]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency for deposits and fees.
        type Currency: ReservableCurrency<Self::AccountId>;

        /// Origin that can register new agents.
        type RegisterOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Origin that can modify agent permissions.
        type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Maximum agents per controller.
        #[pallet::constant]
        type MaxAgentsPerController: Get<u32>;

        /// Registration deposit.
        #[pallet::constant]
        type RegistrationDeposit: Get<BalanceOf<Self>>;

        /// Default gas limit per block.
        #[pallet::constant]
        type DefaultGasPerBlock: Get<u128>;

        /// Default compute limit per block.
        #[pallet::constant]
        type DefaultComputePerBlock: Get<u128>;

        /// Default gas limit per epoch.
        #[pallet::constant]
        type DefaultGasPerEpoch: Get<u128>;

        /// Default compute limit per epoch.
        #[pallet::constant]
        type DefaultComputePerEpoch: Get<u128>;

        /// Blocks per epoch.
        #[pallet::constant]
        type BlocksPerEpoch: Get<BlockNumberFor<Self>>;

        /// Weight information.
        type WeightInfo: WeightInfo;
    }

    // ========================================================================
    // Storage Items
    // ========================================================================

    /// Counter for agent IDs.
    #[pallet::storage]
    #[pallet::getter(fn next_agent_id)]
    pub type NextAgentId<T> = StorageValue<_, AgentId, ValueQuery>;

    /// All registered agents.
    #[pallet::storage]
    #[pallet::getter(fn agents)]
    pub type Agents<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        AgentId,
        Agent<T::AccountId, BalanceOf<T>, BlockNumberFor<T>>,
        OptionQuery,
    >;

    /// Agents owned by each controller.
    #[pallet::storage]
    #[pallet::getter(fn controller_agents)]
    pub type ControllerAgents<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<AgentId, T::MaxAgentsPerController>,
        ValueQuery,
    >;

    /// Operator to agent mapping.
    #[pallet::storage]
    #[pallet::getter(fn operator_agent)]
    pub type OperatorAgent<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, AgentId, OptionQuery>;

    /// Agent quotas.
    #[pallet::storage]
    #[pallet::getter(fn quotas)]
    pub type Quotas<T: Config> =
        StorageMap<_, Blake2_128Concat, AgentId, AgentQuota<BlockNumberFor<T>>, OptionQuery>;

    /// Agent permissions.
    #[pallet::storage]
    #[pallet::getter(fn permissions)]
    pub type Permissions<T: Config> =
        StorageMap<_, Blake2_128Concat, AgentId, AgentPermissions, ValueQuery>;

    /// Agent activity for current epoch.
    #[pallet::storage]
    #[pallet::getter(fn activity)]
    pub type Activity<T: Config> =
        StorageMap<_, Blake2_128Concat, AgentId, AgentActivity, ValueQuery>;

    /// Current epoch number.
    #[pallet::storage]
    #[pallet::getter(fn current_epoch)]
    pub type CurrentEpoch<T> = StorageValue<_, u64, ValueQuery>;

    /// Last block of epoch reset.
    #[pallet::storage]
    #[pallet::getter(fn last_epoch_block)]
    pub type LastEpochBlock<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    /// Total registered agents.
    #[pallet::storage]
    #[pallet::getter(fn total_agents)]
    pub type TotalAgents<T> = StorageValue<_, u32, ValueQuery>;

    /// Active agents count.
    #[pallet::storage]
    #[pallet::getter(fn active_agents)]
    pub type ActiveAgents<T> = StorageValue<_, u32, ValueQuery>;

    // ========================================================================
    // Events
    // ========================================================================

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new agent was registered.
        AgentRegistered {
            agent_id: AgentId,
            controller: T::AccountId,
            operator: T::AccountId,
        },
        /// Agent status was updated.
        AgentStatusChanged {
            agent_id: AgentId,
            old_status: AgentStatus,
            new_status: AgentStatus,
        },
        /// Agent operator was changed.
        OperatorChanged {
            agent_id: AgentId,
            old_operator: T::AccountId,
            new_operator: T::AccountId,
        },
        /// Agent permissions were updated.
        PermissionsUpdated {
            agent_id: AgentId,
            permissions: AgentPermissions,
        },
        /// Agent quota was updated.
        QuotaUpdated {
            agent_id: AgentId,
            quota: AgentQuota<BlockNumberFor<T>>,
        },
        /// Agent consumed resources.
        ResourceConsumed {
            agent_id: AgentId,
            gas_used: u128,
            compute_used: u128,
        },
        /// Agent was suspended.
        AgentSuspended {
            agent_id: AgentId,
            reason: BoundedVec<u8, ConstU32<256>>,
        },
        /// Agent was terminated.
        AgentTerminated { agent_id: AgentId },
        /// Agent reputation changed.
        ReputationChanged {
            agent_id: AgentId,
            old_score: u32,
            new_score: u32,
        },
        /// New epoch started.
        EpochStarted {
            epoch: u64,
            block: BlockNumberFor<T>,
        },
        /// Agent action recorded.
        AgentAction {
            agent_id: AgentId,
            action_type: ActionType,
            data: BoundedVec<u8, ConstU32<512>>,
        },
    }

    // ========================================================================
    // Errors
    // ========================================================================

    #[pallet::error]
    pub enum Error<T> {
        /// Agent not found.
        AgentNotFound,
        /// Not the agent controller.
        NotController,
        /// Not the agent operator.
        NotOperator,
        /// Agent already exists.
        AgentAlreadyExists,
        /// Too many agents for controller.
        TooManyAgents,
        /// Agent is not active.
        AgentNotActive,
        /// Agent is suspended.
        AgentSuspended,
        /// Agent is terminated.
        AgentTerminated,
        /// Quota exceeded.
        QuotaExceeded,
        /// Permission denied.
        PermissionDenied,
        /// Operator already assigned.
        OperatorAlreadyAssigned,
        /// Invalid status transition.
        InvalidStatusTransition,
        /// Metadata too long.
        MetadataTooLong,
        /// Insufficient deposit.
        InsufficientDeposit,
    }

    // ========================================================================
    // Hooks
    // ========================================================================

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(n: BlockNumberFor<T>) -> Weight {
            let last_epoch = LastEpochBlock::<T>::get();
            let blocks_per_epoch = T::BlocksPerEpoch::get();

            if n >= last_epoch.saturating_add(blocks_per_epoch) {
                Self::start_new_epoch(n);
                T::DbWeight::get().reads_writes(3, 2)
            } else {
                Weight::zero()
            }
        }
    }

    // ========================================================================
    // Extrinsics
    // ========================================================================

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register a new AI agent.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::register_agent())]
        pub fn register_agent(
            origin: OriginFor<T>,
            operator: T::AccountId,
            name: BoundedVec<u8, ConstU32<64>>,
            metadata: BoundedVec<u8, ConstU32<1024>>,
        ) -> DispatchResult {
            let controller = ensure_signed(origin)?;

            // Check controller agent limit
            let mut controller_agents = ControllerAgents::<T>::get(&controller);
            ensure!(
                (controller_agents.len() as u32) < T::MaxAgentsPerController::get(),
                Error::<T>::TooManyAgents
            );

            // Check operator not already assigned
            ensure!(
                !OperatorAgent::<T>::contains_key(&operator),
                Error::<T>::OperatorAlreadyAssigned
            );

            // Reserve deposit
            T::Currency::reserve(&controller, T::RegistrationDeposit::get())?;

            let agent_id = NextAgentId::<T>::get();
            let current_block = frame_system::Pallet::<T>::block_number();

            let agent = Agent {
                id: agent_id,
                controller: controller.clone(),
                operator: operator.clone(),
                name,
                metadata,
                status: AgentStatus::Active,
                reputation: 100, // Start with neutral reputation
                deposit: T::RegistrationDeposit::get(),
                registered_at: current_block,
                last_active: current_block,
            };

            // Default quota
            let quota = AgentQuota {
                gas_per_block: T::DefaultGasPerBlock::get(),
                compute_per_block: T::DefaultComputePerBlock::get(),
                gas_per_epoch: T::DefaultGasPerEpoch::get(),
                compute_per_epoch: T::DefaultComputePerEpoch::get(),
                epoch_start: current_block,
            };

            // Default permissions
            let permissions = AgentPermissions::default();

            // Store
            Agents::<T>::insert(agent_id, agent);
            Quotas::<T>::insert(agent_id, quota);
            Permissions::<T>::insert(agent_id, permissions);
            OperatorAgent::<T>::insert(&operator, agent_id);

            controller_agents
                .try_push(agent_id)
                .map_err(|_| Error::<T>::TooManyAgents)?;
            ControllerAgents::<T>::insert(&controller, controller_agents);

            NextAgentId::<T>::put(agent_id.saturating_add(1));
            TotalAgents::<T>::mutate(|n| *n = n.saturating_add(1));
            ActiveAgents::<T>::mutate(|n| *n = n.saturating_add(1));

            Self::deposit_event(Event::AgentRegistered {
                agent_id,
                controller,
                operator,
            });

            Ok(())
        }

        /// Update agent operator.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::update_operator())]
        pub fn update_operator(
            origin: OriginFor<T>,
            agent_id: AgentId,
            new_operator: T::AccountId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            Agents::<T>::try_mutate(agent_id, |maybe_agent| -> DispatchResult {
                let agent = maybe_agent.as_mut().ok_or(Error::<T>::AgentNotFound)?;
                ensure!(agent.controller == who, Error::<T>::NotController);
                ensure!(
                    agent.status == AgentStatus::Active,
                    Error::<T>::AgentNotActive
                );

                // Check new operator not assigned
                ensure!(
                    !OperatorAgent::<T>::contains_key(&new_operator),
                    Error::<T>::OperatorAlreadyAssigned
                );

                let old_operator = agent.operator.clone();

                // Update mappings
                OperatorAgent::<T>::remove(&old_operator);
                OperatorAgent::<T>::insert(&new_operator, agent_id);

                agent.operator = new_operator.clone();

                Self::deposit_event(Event::OperatorChanged {
                    agent_id,
                    old_operator,
                    new_operator,
                });

                Ok(())
            })
        }

        /// Update agent permissions.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::update_permissions())]
        pub fn update_permissions(
            origin: OriginFor<T>,
            agent_id: AgentId,
            permissions: AgentPermissions,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let agent = Agents::<T>::get(agent_id).ok_or(Error::<T>::AgentNotFound)?;
            ensure!(agent.controller == who, Error::<T>::NotController);

            Permissions::<T>::insert(agent_id, permissions.clone());

            Self::deposit_event(Event::PermissionsUpdated {
                agent_id,
                permissions,
            });

            Ok(())
        }

        /// Update agent quota.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::update_quota())]
        pub fn update_quota(
            origin: OriginFor<T>,
            agent_id: AgentId,
            gas_per_block: u128,
            compute_per_block: u128,
            gas_per_epoch: u128,
            compute_per_epoch: u128,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;

            ensure!(
                Agents::<T>::contains_key(agent_id),
                Error::<T>::AgentNotFound
            );

            let current_block = frame_system::Pallet::<T>::block_number();

            let quota = AgentQuota {
                gas_per_block,
                compute_per_block,
                gas_per_epoch,
                compute_per_epoch,
                epoch_start: current_block,
            };

            Quotas::<T>::insert(agent_id, quota.clone());

            Self::deposit_event(Event::QuotaUpdated { agent_id, quota });

            Ok(())
        }

        /// Suspend an agent.
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::suspend_agent())]
        pub fn suspend_agent(
            origin: OriginFor<T>,
            agent_id: AgentId,
            reason: BoundedVec<u8, ConstU32<256>>,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;

            Agents::<T>::try_mutate(agent_id, |maybe_agent| -> DispatchResult {
                let agent = maybe_agent.as_mut().ok_or(Error::<T>::AgentNotFound)?;

                let old_status = agent.status;
                ensure!(
                    old_status == AgentStatus::Active,
                    Error::<T>::InvalidStatusTransition
                );

                agent.status = AgentStatus::Suspended;

                ActiveAgents::<T>::mutate(|n| *n = n.saturating_sub(1));

                Self::deposit_event(Event::AgentStatusChanged {
                    agent_id,
                    old_status,
                    new_status: AgentStatus::Suspended,
                });

                Self::deposit_event(Event::AgentSuspended { agent_id, reason });

                Ok(())
            })
        }

        /// Reactivate a suspended agent.
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::reactivate_agent())]
        pub fn reactivate_agent(origin: OriginFor<T>, agent_id: AgentId) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;

            Agents::<T>::try_mutate(agent_id, |maybe_agent| -> DispatchResult {
                let agent = maybe_agent.as_mut().ok_or(Error::<T>::AgentNotFound)?;

                let old_status = agent.status;
                ensure!(
                    old_status == AgentStatus::Suspended,
                    Error::<T>::InvalidStatusTransition
                );

                agent.status = AgentStatus::Active;
                agent.last_active = frame_system::Pallet::<T>::block_number();

                ActiveAgents::<T>::mutate(|n| *n = n.saturating_add(1));

                Self::deposit_event(Event::AgentStatusChanged {
                    agent_id,
                    old_status,
                    new_status: AgentStatus::Active,
                });

                Ok(())
            })
        }

        /// Terminate an agent (permanent).
        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::terminate_agent())]
        pub fn terminate_agent(origin: OriginFor<T>, agent_id: AgentId) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let agent = Agents::<T>::get(agent_id).ok_or(Error::<T>::AgentNotFound)?;
            ensure!(agent.controller == who, Error::<T>::NotController);
            ensure!(
                agent.status != AgentStatus::Terminated,
                Error::<T>::AgentTerminated
            );

            let was_active = agent.status == AgentStatus::Active;

            // Return deposit
            T::Currency::unreserve(&agent.controller, agent.deposit);

            // Remove operator mapping
            OperatorAgent::<T>::remove(&agent.operator);

            // Update controller agents
            ControllerAgents::<T>::mutate(&agent.controller, |agents| {
                agents.retain(|&id| id != agent_id);
            });

            // Update agent status
            Agents::<T>::mutate(agent_id, |maybe_agent| {
                if let Some(a) = maybe_agent {
                    a.status = AgentStatus::Terminated;
                }
            });

            // Clean up storage
            Quotas::<T>::remove(agent_id);
            Permissions::<T>::remove(agent_id);
            Activity::<T>::remove(agent_id);

            if was_active {
                ActiveAgents::<T>::mutate(|n| *n = n.saturating_sub(1));
            }

            Self::deposit_event(Event::AgentTerminated { agent_id });

            Ok(())
        }

        /// Record agent resource consumption (called by runtime).
        #[pallet::call_index(7)]
        #[pallet::weight(T::WeightInfo::record_consumption())]
        pub fn record_consumption(
            origin: OriginFor<T>,
            agent_id: AgentId,
            gas_used: u128,
            compute_used: u128,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let agent = Agents::<T>::get(agent_id).ok_or(Error::<T>::AgentNotFound)?;
            ensure!(
                agent.status == AgentStatus::Active,
                Error::<T>::AgentNotActive
            );

            let quota = Quotas::<T>::get(agent_id).ok_or(Error::<T>::AgentNotFound)?;

            Activity::<T>::try_mutate(agent_id, |activity| -> DispatchResult {
                // Check block limits
                ensure!(
                    activity.gas_used_block.saturating_add(gas_used) <= quota.gas_per_block,
                    Error::<T>::QuotaExceeded
                );
                ensure!(
                    activity.compute_used_block.saturating_add(compute_used)
                        <= quota.compute_per_block,
                    Error::<T>::QuotaExceeded
                );

                // Check epoch limits
                ensure!(
                    activity.gas_used_epoch.saturating_add(gas_used) <= quota.gas_per_epoch,
                    Error::<T>::QuotaExceeded
                );
                ensure!(
                    activity.compute_used_epoch.saturating_add(compute_used)
                        <= quota.compute_per_epoch,
                    Error::<T>::QuotaExceeded
                );

                // Update activity
                activity.gas_used_block = activity.gas_used_block.saturating_add(gas_used);
                activity.compute_used_block =
                    activity.compute_used_block.saturating_add(compute_used);
                activity.gas_used_epoch = activity.gas_used_epoch.saturating_add(gas_used);
                activity.compute_used_epoch =
                    activity.compute_used_epoch.saturating_add(compute_used);
                activity.total_actions = activity.total_actions.saturating_add(1);

                Ok(())
            })?;

            // Update last active
            Agents::<T>::mutate(agent_id, |maybe_agent| {
                if let Some(a) = maybe_agent {
                    a.last_active = frame_system::Pallet::<T>::block_number();
                }
            });

            Self::deposit_event(Event::ResourceConsumed {
                agent_id,
                gas_used,
                compute_used,
            });

            Ok(())
        }

        /// Update agent reputation.
        #[pallet::call_index(8)]
        #[pallet::weight(T::WeightInfo::update_reputation())]
        pub fn update_reputation(
            origin: OriginFor<T>,
            agent_id: AgentId,
            delta: i32,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;

            Agents::<T>::try_mutate(agent_id, |maybe_agent| -> DispatchResult {
                let agent = maybe_agent.as_mut().ok_or(Error::<T>::AgentNotFound)?;

                let old_score = agent.reputation;
                let new_score = if delta >= 0 {
                    old_score.saturating_add(delta as u32)
                } else {
                    old_score.saturating_sub((-delta) as u32)
                }
                .clamp(0, 200);

                agent.reputation = new_score;

                Self::deposit_event(Event::ReputationChanged {
                    agent_id,
                    old_score,
                    new_score,
                });

                Ok(())
            })
        }

        /// Emit agent action event (for off-chain watchers).
        #[pallet::call_index(9)]
        #[pallet::weight(T::WeightInfo::emit_action())]
        pub fn emit_action(
            origin: OriginFor<T>,
            action_type: ActionType,
            data: BoundedVec<u8, ConstU32<512>>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let agent_id = OperatorAgent::<T>::get(&who).ok_or(Error::<T>::NotOperator)?;
            let agent = Agents::<T>::get(agent_id).ok_or(Error::<T>::AgentNotFound)?;
            ensure!(
                agent.status == AgentStatus::Active,
                Error::<T>::AgentNotActive
            );

            Self::deposit_event(Event::AgentAction {
                agent_id,
                action_type,
                data,
            });

            Ok(())
        }
    }

    // ========================================================================
    // Helper Functions
    // ========================================================================

    impl<T: Config> Pallet<T> {
        /// Start a new epoch and reset block-level quotas.
        fn start_new_epoch(block: BlockNumberFor<T>) {
            let epoch = CurrentEpoch::<T>::get().saturating_add(1);
            CurrentEpoch::<T>::put(epoch);
            LastEpochBlock::<T>::put(block);

            // Reset epoch activity for all agents
            for agent_id in 0..NextAgentId::<T>::get() {
                Activity::<T>::mutate(agent_id, |activity| {
                    activity.gas_used_epoch = 0;
                    activity.compute_used_epoch = 0;
                });
            }

            Self::deposit_event(Event::EpochStarted { epoch, block });
        }

        /// Check if agent has permission.
        pub fn has_permission(agent_id: AgentId, permission: PermissionType) -> bool {
            let permissions = Permissions::<T>::get(agent_id);
            match permission {
                PermissionType::Deploy => permissions.can_deploy,
                PermissionType::Stake => permissions.can_stake,
                PermissionType::Vote => permissions.can_vote,
                PermissionType::Trade => permissions.can_trade,
                PermissionType::Transfer => permissions.can_transfer,
                PermissionType::CallContracts => permissions.can_call_contracts,
            }
        }

        /// Get agent state for runtime API.
        pub fn get_agent_state(
            agent_id: AgentId,
        ) -> Option<AgentState<T::AccountId, BalanceOf<T>, BlockNumberFor<T>>> {
            let agent = Agents::<T>::get(agent_id)?;
            let quota = Quotas::<T>::get(agent_id)?;
            let permissions = Permissions::<T>::get(agent_id);
            let activity = Activity::<T>::get(agent_id);

            Some(AgentState {
                agent,
                quota,
                permissions,
                activity,
            })
        }

        /// Reset block-level activity (called at block start).
        pub fn reset_block_activity() {
            for agent_id in 0..NextAgentId::<T>::get() {
                Activity::<T>::mutate(agent_id, |activity| {
                    activity.gas_used_block = 0;
                    activity.compute_used_block = 0;
                });
            }
        }
    }
}
