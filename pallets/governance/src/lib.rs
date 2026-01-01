#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]


#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_std::collections::btree_map::BTreeMap;
    use sp_std::vec::Vec;

    pub type ProposalId = u32;

    /// Proposal status
    #[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
    pub enum ProposalStatus {
        Pending,      // Awaiting votes
        Approved,     // Passed voting, ready to execute
        Executed,     // Executed successfully
        Rejected,     // Failed voting
        Cancelled,    // Cancelled by proposer
    }

    /// Governance action type
    #[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
    pub enum GovernanceAction {
        UpdateParameter(Vec<u8>, u128),    // Parameter name, new value
        SetFlashLoanFee(u32),              // New fee in bps
        SetOracleFee(u32),                 // New oracle fee
        AgentWhitelist(Vec<u8>),           // Agent address to whitelist
        TreasuryTransfer(u128),            // Amount to transfer
        EmergencyPause,                    // Pause all operations
        ResumeOperations,                  // Resume after pause
    }

    /// Governance proposal
    #[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct Proposal<T: Config> {
        pub proposal_id: ProposalId,
        pub proposer: T::AccountId,
        pub title: Vec<u8>,
        pub description: Vec<u8>,
        pub action: GovernanceAction,
        pub voting_period: BlockNumberFor<T>,
        pub vote_threshold: u32,           // bps (e.g., 6600 = 66%)
        pub votes_for: u32,
        pub votes_against: u32,
        pub status: ProposalStatus,
        pub created_block: BlockNumberFor<T>,
        pub end_block: BlockNumberFor<T>,
    }

    /// Vote on a proposal
    #[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
    pub enum Vote {
        Yes,
        No,
    }

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Default voting period in blocks
        #[pallet::constant]
        type DefaultVotingPeriod: Get<BlockNumberFor<Self>>;

        /// Default vote threshold (bps)
        #[pallet::constant]
        type DefaultVoteThreshold: Get<u32>;

        /// Timelock before executing approved proposals (blocks)
        #[pallet::constant]
        type TimelockBlocks: Get<BlockNumberFor<Self>>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    /// All proposals
    #[pallet::storage]
    #[pallet::getter(fn proposal)]
    pub type Proposals<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        ProposalId,
        Proposal<T>,
        OptionQuery,
    >;

    /// Votes for each proposal
    #[pallet::storage]
    #[pallet::getter(fn votes)]
    pub type ProposalVotes<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        (ProposalId, T::AccountId),
        Vote,
        OptionQuery,
    >;

    /// Proposal counter
    #[pallet::storage]
    #[pallet::getter(fn proposal_counter)]
    pub type ProposalCounter<T: Config> = StorageValue<_, ProposalId, ValueQuery>;

    /// Agent reputation scores
    #[pallet::storage]
    #[pallet::getter(fn agent_reputation)]
    pub type AgentReputation<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        u32,  // Reputation score 0-1000
        ValueQuery,
    >;

    /// Governance parameters (governance-controlled)
    #[pallet::storage]
    #[pallet::getter(fn parameter)]
    pub type GovernanceParameters<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        Vec<u8>,  // Parameter name
        u128,     // Parameter value
        ValueQuery,
    >;

    /// Protocol pause state
    #[pallet::storage]
    #[pallet::getter(fn is_paused)]
    pub type ProtocolPaused<T: Config> = StorageValue<_, bool, ValueQuery>;

    /// Total executed proposals
    #[pallet::storage]
    #[pallet::getter(fn executed_proposals)]
    pub type ExecutedProposals<T: Config> = StorageValue<_, u32, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Proposal submitted
        ProposalSubmitted {
            proposal_id: ProposalId,
            proposer: T::AccountId,
            title: Vec<u8>,
        },
        /// Vote cast on proposal
        VoteCasted {
            proposal_id: ProposalId,
            voter: T::AccountId,
            vote: Vote,
        },
        /// Proposal passed voting
        ProposalPassed {
            proposal_id: ProposalId,
            votes_for: u32,
            votes_against: u32,
        },
        /// Proposal rejected in voting
        ProposalRejected {
            proposal_id: ProposalId,
            votes_for: u32,
            votes_against: u32,
        },
        /// Proposal executed
        ProposalExecuted {
            proposal_id: ProposalId,
            action: Vec<u8>,
        },
        /// Governance parameter updated
        ParameterUpdated {
            param_name: Vec<u8>,
            old_value: u128,
            new_value: u128,
        },
        /// Protocol paused
        ProtocolPaused {
            reason: Vec<u8>,
        },
        /// Protocol resumed
        ProtocolResumed,
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Proposal not found
        ProposalNotFound,
        /// Voting period not ended
        VotingActive,
        /// Voting period ended
        VotingEnded,
        /// Agent not found
        AgentNotFound,
        /// Already voted
        AlreadyVoted,
        /// Invalid proposal action
        InvalidAction,
        /// Insufficient reputation
        InsufficientReputation,
        /// Protocol is paused
        ProtocolPaused,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_finalize(_n: BlockNumberFor<T>) {
            // Finalize completed proposals and check thresholds
            let current_block = frame_system::Pallet::<T>::block_number();

            Proposals::<T>::iter_mut().for_each(|(_, mut proposal)| {
                // Check if voting period ended
                if current_block > proposal.end_block && proposal.status == ProposalStatus::Pending {
                    let total_votes = proposal.votes_for.saturating_add(proposal.votes_against);
                    
                    if total_votes > 0 {
                        let threshold_votes = (total_votes * proposal.vote_threshold) / 10_000u32;
                        
                        if proposal.votes_for >= threshold_votes {
                            proposal.status = ProposalStatus::Approved;
                            Self::deposit_event(Event::ProposalPassed {
                                proposal_id: proposal.proposal_id,
                                votes_for: proposal.votes_for,
                                votes_against: proposal.votes_against,
                            });
                        } else {
                            proposal.status = ProposalStatus::Rejected;
                            Self::deposit_event(Event::ProposalRejected {
                                proposal_id: proposal.proposal_id,
                                votes_for: proposal.votes_for,
                                votes_against: proposal.votes_against,
                            });
                        }
                    }
                    
                    Proposals::<T>::insert(proposal.proposal_id, &proposal);
                }
            });
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Submit a governance proposal
        #[pallet::call_index(0)]
        #[pallet::weight(10_000)]
        pub fn submit_proposal(
            origin: OriginFor<T>,
            title: Vec<u8>,
            description: Vec<u8>,
            action: GovernanceAction,
        ) -> DispatchResult {
            let proposer = ensure_signed(origin)?;
            ensure!(!ProtocolPaused::<T>::get(), Error::<T>::ProtocolPaused);

            // Check proposer reputation
            let reputation = AgentReputation::<T>::get(&proposer);
            ensure!(reputation >= 100u32, Error::<T>::InsufficientReputation);

            // Generate proposal ID
            let proposal_id = Self::next_proposal_id();

            let current_block = frame_system::Pallet::<T>::block_number();
            let end_block = current_block.saturating_add(T::DefaultVotingPeriod::get());

            let proposal = Proposal {
                proposal_id,
                proposer: proposer.clone(),
                title: title.clone(),
                description,
                action,
                voting_period: T::DefaultVotingPeriod::get(),
                vote_threshold: T::DefaultVoteThreshold::get(),
                votes_for: 0,
                votes_against: 0,
                status: ProposalStatus::Pending,
                created_block: current_block,
                end_block,
            };

            Proposals::<T>::insert(proposal_id, proposal);

            Self::deposit_event(Event::ProposalSubmitted {
                proposal_id,
                proposer,
                title,
            });

            Ok(())
        }

        /// Cast vote on proposal
        #[pallet::call_index(1)]
        #[pallet::weight(5_000)]
        pub fn vote(
            origin: OriginFor<T>,
            proposal_id: ProposalId,
            vote: Vote,
        ) -> DispatchResult {
            let voter = ensure_signed(origin)?;
            ensure!(!ProtocolPaused::<T>::get(), Error::<T>::ProtocolPaused);

            // Check proposal exists
            let mut proposal = Self::proposal(proposal_id).ok_or(Error::<T>::ProposalNotFound)?;
            ensure!(proposal.status == ProposalStatus::Pending, Error::<T>::VotingEnded);

            let current_block = frame_system::Pallet::<T>::block_number();
            ensure!(current_block <= proposal.end_block, Error::<T>::VotingEnded);

            // Check not already voted
            ensure!(
                ProposalVotes::<T>::get((proposal_id, &voter)).is_none(),
                Error::<T>::AlreadyVoted
            );

            // Get voter reputation (used for vote weight)
            let reputation = AgentReputation::<T>::get(&voter);
            let vote_weight = if reputation > 500u32 { 2u32 } else { 1u32 };

            // Record vote
            ProposalVotes::<T>::insert((proposal_id, &voter), vote.clone());

            // Update vote counts (with reputation weighting)
            match vote {
                Vote::Yes => proposal.votes_for = proposal.votes_for.saturating_add(vote_weight),
                Vote::No => proposal.votes_against = proposal.votes_against.saturating_add(vote_weight),
            }

            Proposals::<T>::insert(proposal_id, proposal);

            Self::deposit_event(Event::VoteCasted {
                proposal_id,
                voter,
                vote,
            });

            Ok(())
        }

        /// Execute approved proposal (after timelock)
        #[pallet::call_index(2)]
        #[pallet::weight(20_000)]
        pub fn execute_proposal(
            origin: OriginFor<T>,
            proposal_id: ProposalId,
        ) -> DispatchResult {
            let _executor = ensure_signed(origin)?;

            let mut proposal = Self::proposal(proposal_id).ok_or(Error::<T>::ProposalNotFound)?;
            ensure!(proposal.status == ProposalStatus::Approved, Error::<T>::ProposalNotFound);

            // Check timelock
            let current_block = frame_system::Pallet::<T>::block_number();
            let timelock_block = proposal.end_block.saturating_add(T::TimelockBlocks::get());
            ensure!(current_block >= timelock_block, Error::<T>::VotingActive);

            // Execute action
            Self::execute_action(&proposal.action)?;

            proposal.status = ProposalStatus::Executed;
            Proposals::<T>::insert(proposal_id, proposal);

            let executed = ExecutedProposals::<T>::get();
            ExecutedProposals::<T>::put(executed.saturating_add(1));

            Self::deposit_event(Event::ProposalExecuted {
                proposal_id,
                action: b"executed".to_vec(),
            });

            Ok(())
        }

        /// Set agent reputation (governance call)
        #[pallet::call_index(3)]
        #[pallet::weight(5_000)]
        pub fn set_agent_reputation(
            origin: OriginFor<T>,
            agent: T::AccountId,
            reputation: u32,
        ) -> DispatchResult {
            let _root = ensure_root(origin)?;
            AgentReputation::<T>::insert(agent, reputation);
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        fn next_proposal_id() -> ProposalId {
            let counter = ProposalCounter::<T>::get();
            let new_counter = counter.saturating_add(1);
            ProposalCounter::<T>::put(new_counter);
            new_counter
        }

        fn execute_action(action: &GovernanceAction) -> DispatchResult {
            match action {
                GovernanceAction::UpdateParameter(param_name, new_value) => {
                    let old_value = GovernanceParameters::<T>::get(param_name);
                    GovernanceParameters::<T>::insert(param_name, new_value);
                    
                    // Emit event (would be captured by RPC)
                    Ok(())
                },
                GovernanceAction::SetFlashLoanFee(fee_bps) => {
                    GovernanceParameters::<T>::insert(b"FLASH_LOAN_FEE_BPS".to_vec(), *fee_bps as u128);
                    Ok(())
                },
                GovernanceAction::SetOracleFee(fee_bps) => {
                    GovernanceParameters::<T>::insert(b"ORACLE_FEE_BPS".to_vec(), *fee_bps as u128);
                    Ok(())
                },
                GovernanceAction::EmergencyPause => {
                    ProtocolPaused::<T>::put(true);
                    Ok(())
                },
                GovernanceAction::ResumeOperations => {
                    ProtocolPaused::<T>::put(false);
                    Ok(())
                },
                _ => Ok(()),
            }
        }

        pub fn get_parameter(name: &[u8]) -> u128 {
            GovernanceParameters::<T>::get(name)
        }

        pub fn get_stats() -> (u32, u32) {
            (ProposalCounter::<T>::get(), ExecutedProposals::<T>::get())
        }
    }
}
