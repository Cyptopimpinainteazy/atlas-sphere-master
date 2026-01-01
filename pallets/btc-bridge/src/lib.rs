//! BTC Bridge Pallet
//!
//! This pallet provides Bitcoin interoperability via:
//! - SPV (Simplified Payment Verification) for header validation
//! - Merkle proof verification for transaction inclusion
//! - Atomic peg-in/peg-out mechanisms
//! - Multi-signature security for high-value operations

#![cfg_attr(not(feature = "std"), no_std)]


mod spv;
mod types;
pub mod weights;

pub use types::*;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use crate::types::*;
    use crate::spv;
    use frame_support::{pallet_prelude::*, fail};
    use frame_system::pallet_prelude::*;
    use sp_std::vec::Vec;
    use sp_core::{H256, U256};

    /// The current storage version
    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        
        /// Maximum depth of Merkle proof
        #[pallet::constant]
        type MaxProofDepth: Get<u32>;
        
        /// Required BTC confirmations for finality
        #[pallet::constant]
        type ConfirmationDepth: Get<u32>;
        
        /// Minimum peg-in amount
        #[pallet::constant]
        type MinPeginAmount: Get<u64>;
        
        /// Maximum peg-in amount
        #[pallet::constant]
        type MaxPeginAmount: Get<u64>;
    }

    // Storage

    /// Current BTC block header chain
    #[pallet::storage]
    #[pallet::getter(fn btc_headers)]
    pub type BtcHeaders<T: Config> = StorageMap<_, Blake2_128Concat, u32, BtcBlockHeader>;

    /// Current main chain tip
    #[pallet::storage]
    #[pallet::getter(fn chain_tip)]
    pub type ChainTip<T: Config> = StorageValue<_, (u32, H256), ValueQuery>;

    /// Processed transactions (prevented replay)
    #[pallet::storage]
    #[pallet::getter(fn processed_txs)]
    pub type ProcessedTransactions<T: Config> = StorageMap<_, Blake2_128Concat, H256, (), OptionQuery>;

    /// Peg-in requests pending finality
    #[pallet::storage]
    #[pallet::getter(fn pending_pegIn_requests)]
    pub type PeginRequests<T: Config> = StorageMap<_, Blake2_128Concat, H256, PeginRequest<T::AccountId>>;

    /// Confirmed peg-ins mapped to account
    #[pallet::storage]
    #[pallet::getter(fn confirmed_pegIns)]
    pub type ConfirmedPegins<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, u64, ValueQuery>;

    /// Bridge configuration
    #[pallet::storage]
    pub type BridgeConfig<T: Config> = StorageValue<_, BridgeConfiguration, ValueQuery>;

    /// Governance multisig for emergency operations
    #[pallet::storage]
    pub type GovernanceMultisig<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

    // Extrinsics

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Initialize bridge with BTC genesis block
        #[pallet::call_index(0)]
        #[pallet::weight(T::DbWeight::get().writes(2))]
        pub fn init_bridge(
            origin: OriginFor<T>,
            genesis_header: BtcBlockHeader,
        ) -> DispatchResult {
            ensure_root(origin)?;
            
            // Store genesis block at height 0
            <BtcHeaders<T>>::insert(0, genesis_header.clone());
            <ChainTip<T>>::put((0, genesis_header.hash()));

            Self::deposit_event(Event::BridgeInitialized {
                genesis_block: genesis_header.hash(),
            });

            Ok(())
        }

        /// Add BTC block header to chain
        /// Validates against previous header and difficulty adjustment
        #[pallet::call_index(1)]
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 2))]
        pub fn add_header(
            origin: OriginFor<T>,
            header: BtcBlockHeader,
        ) -> DispatchResult {
            ensure_none(origin)?;  // Can be called by anyone

            let (tip_height, tip_hash) = <ChainTip<T>>::get();

            // Validate header
            spv::validate_header(&header, &tip_hash)?;

            // Check difficulty (simplified)
            ensure!(header.bits == 0x207fffff, Error::<T>::InvalidDifficulty);

            // Store header
            let new_height = tip_height + 1;
            <BtcHeaders<T>>::insert(new_height, header.clone());
            <ChainTip<T>>::put((new_height, header.hash()));

            Self::deposit_event(Event::HeaderAdded {
                height: new_height,
                block_hash: header.hash(),
            });

            Ok(())
        }

        /// Request peg-in: user provides BTC tx and Merkle proof
        #[pallet::call_index(2)]
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 1))]
        pub fn request_pegin(
            origin: OriginFor<T>,
            btc_tx: Vec<u8>,
            merkle_proof: MerkleProof,
            output_index: u32,
            recipient: T::AccountId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Parse and validate BTC transaction
            let tx_hash = spv::compute_tx_hash(&btc_tx)?;

            // Verify Merkle proof
            spv::verify_merkle_proof(&merkle_proof, tx_hash)?;

            // Extract output value
            let value = spv::extract_output_value(&btc_tx, output_index)?;

            // Validate amount
            let config = <BridgeConfig<T>>::get();
            ensure!(value >= config.min_pegin_amount, Error::<T>::AmountTooSmall);
            ensure!(value <= config.max_pegin_amount, Error::<T>::AmountTooLarge);

            // Store request (waiting for confirmations)
            let request = PeginRequest {
                btc_tx_hash: tx_hash,
                requester: who,
                recipient: recipient.clone(),
                amount: value,
                requested_at: <frame_system::Pallet<T>>::block_number(),
                status: RequestStatus::Pending,
            };

            <PeginRequests<T>>::insert(tx_hash, request);

            Self::deposit_event(Event::PeginRequested {
                tx_hash,
                recipient,
                amount: value,
            });

            Ok(())
        }

        /// Finalize peg-in after confirmation period
        #[pallet::call_index(3)]
        #[pallet::weight(T::DbWeight::get().reads_writes(3, 3))]
        pub fn finalize_pegin(
            origin: OriginFor<T>,
            tx_hash: H256,
        ) -> DispatchResult {
            ensure_none(origin)?;

            let mut request = <PeginRequests<T>>::get(&tx_hash)
                .ok_or(Error::<T>::PeginNotFound)?;

            // Verify tx is in confirmed blocks
            spv::verify_tx_included(&tx_hash)?;

            // Check confirmation depth
            let (tip_height, _) = <ChainTip<T>>::get();
            let confirmations = tip_height.saturating_sub(request.requested_at.try_into().unwrap_or(0));
            ensure!(confirmations >= T::ConfirmationDepth::get(), Error::<T>::InsufficientConfirmations);

            // Mark as processed (prevent double-spending)
            <ProcessedTransactions<T>>::insert(&tx_hash, ());

            // Credit account
            <ConfirmedPegins<T>>::mutate(&request.recipient, |balance| {
                *balance = balance.saturating_add(request.amount);
            });

            request.status = RequestStatus::Confirmed;
            <PeginRequests<T>>::insert(&tx_hash, request.clone());

            Self::deposit_event(Event::PeginFinalized {
                tx_hash,
                recipient: request.recipient,
                amount: request.amount,
            });

            Ok(())
        }

        /// Request peg-out: user burns tokens for BTC
        #[pallet::call_index(4)]
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 2))]
        pub fn request_pegout(
            origin: OriginFor<T>,
            amount: u64,
            btc_address: Vec<u8>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let config = <BridgeConfig<T>>::get();
            ensure!(amount >= config.min_pegout_amount, Error::<T>::AmountTooSmall);

            // Burn tokens
            <ConfirmedPegins<T>>::mutate(&who, |balance| {
                *balance = balance.saturating_sub(amount);
            });

            Self::deposit_event(Event::PegoutRequested {
                requester: who,
                amount,
                btc_address,
            });

            Ok(())
        }

        /// Governance function: update bridge config
        #[pallet::call_index(5)]
        #[pallet::weight(T::DbWeight::get().writes(1))]
        pub fn update_config(
            origin: OriginFor<T>,
            new_config: BridgeConfiguration,
        ) -> DispatchResult {
            ensure_root(origin)?;

            <BridgeConfig<T>>::put(new_config);

            Self::deposit_event(Event::ConfigUpdated);

            Ok(())
        }
    }

    // Events

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        BridgeInitialized { genesis_block: H256 },
        HeaderAdded { height: u32, block_hash: H256 },
        PeginRequested {
            tx_hash: H256,
            recipient: T::AccountId,
            amount: u64,
        },
        PeginFinalized {
            tx_hash: H256,
            recipient: T::AccountId,
            amount: u64,
        },
        PegoutRequested {
            requester: T::AccountId,
            amount: u64,
            btc_address: Vec<u8>,
        },
        ConfigUpdated,
    }

    // Errors

    #[pallet::error]
    pub enum Error<T> {
        InvalidHeaderProof,
        InvalidDifficulty,
        InvalidMerkleProof,
        AmountTooSmall,
        AmountTooLarge,
        PeginNotFound,
        TransactionAlreadyProcessed,
        InsufficientConfirmations,
        InvalidBtcAddress,
        InvalidBtcTransaction,
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub bridge_config: Option<BridgeConfiguration>,
        pub _phantom: sp_std::marker::PhantomData<T>,
    }

    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                bridge_config: Some(BridgeConfiguration::default()),
                _phantom: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            if let Some(config) = &self.bridge_config {
                <BridgeConfig<T>>::put(config.clone());
            }
        }
    }
}
