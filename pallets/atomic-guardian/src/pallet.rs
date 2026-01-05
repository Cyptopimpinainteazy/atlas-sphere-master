#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

use sp_std::vec::Vec;

use codec::{Decode, Encode};
use frame_support::{
    pallet_prelude::*,
    traits::Get,
};
use frame_system::pallet_prelude::*;
use scale_info::TypeInfo;
use sp_core::{sr25519, Pair as _};
use sp_runtime::transaction_validity::{
    InvalidTransaction, TransactionSource, TransactionValidity, ValidTransaction,
};

/// Blake2-256 hash helper (no_std compatible)
fn blake2_256(data: &[u8]) -> [u8; 32] {
    sp_core::hashing::blake2_256(data)
}

/// sr25519 signature verification (native-compatible: uses Pair::verify instead of sp_io)
fn sr25519_verify(sig: &sr25519::Signature, msg: &[u8], pk: &sr25519::Public) -> bool {
    sr25519::Pair::verify(sig, msg, pk)
}

use asga_receipts::{
    AttestationScheme, AttestedReceipt, DomainId, Phase, ReceiptPayload,
};

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>>
            + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Maximum number of external domains that can participate in an intent.
        #[pallet::constant]
        type MaxDomains: Get<u32>;

        /// Maximum size (bytes) of a SCALE-encoded canonical receipt stored on-chain.
        #[pallet::constant]
        type MaxReceiptBytes: Get<u32>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct Intent<T: Config> {
        pub proposer: T::AccountId,
        pub required_domains: BoundedVec<DomainId, T::MaxDomains>,
        pub created_at: BlockNumberFor<T>,
        pub expires_at: BlockNumberFor<T>,
    }

    #[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum SwapStatus {
        Open,
        Completed,
        Reverted,
    }

    #[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub struct Progress {
        pub phase: Phase,
        pub status: SwapStatus,
    }

    impl Default for Progress {
        fn default() -> Self {
            Self { phase: Phase::Lock, status: SwapStatus::Open }
        }
    }

    #[pallet::storage]
    #[pallet::getter(fn intents)]
    pub type Intents<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        [u8; 32],
        Intent<T>,
        OptionQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn progress)]
    pub type ProgressByIntent<T: Config> =
        StorageMap<_, Blake2_128Concat, [u8; 32], Progress, ValueQuery>;

    /// Stored canonical receipt bytes indexed by (intent_id, (domain, phase)).
    #[pallet::storage]
    #[pallet::getter(fn receipt_bytes)]
    pub type ReceiptBytes<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        [u8; 32],
        Blake2_128Concat,
        (DomainId, Phase),
        BoundedVec<u8, T::MaxReceiptBytes>,
        OptionQuery,
    >;

    /// Registered attesters (sr25519 pubkey bytes).
    #[pallet::storage]
    #[pallet::getter(fn attester_registered)]
    pub type RegisteredAttesters<T: Config> =
        StorageMap<_, Blake2_128Concat, [u8; 32], bool, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        IntentSubmitted([u8; 32], T::AccountId),
        AttesterRegistered([u8; 32]),
        AttesterUnregistered([u8; 32]),
        ReceiptAccepted([u8; 32], DomainId, Phase, [u8; 32]),
        PhaseAdvanced([u8; 32], Phase),
        Completed([u8; 32]),
        Reverted([u8; 32]),
        /// An invariant was enforced (e.g., timeout triggered automatic revert)
        InvariantEnforced([u8; 32], InvariantType),
    }

    #[pallet::error]
    pub enum Error<T> {
        IntentAlreadyExists,
        IntentNotFound,
        IntentExpired,
        TooManyDomains,
        AttesterNotRegistered,
        BadAttestation,
        PayloadDomainMismatch,
        ReceiptAlreadySubmitted,
        ReceiptTooLarge,
        InvalidPhase,
        AlreadyFinalized,
    }

    #[pallet::validate_unsigned]
    impl<T: Config> ValidateUnsigned for Pallet<T> {
        type Call = Call<T>;

        fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            match call {
                Call::submit_attested_receipt_unsigned { attested } => {
                    // Basic anti-spam checks: require a known scheme and correct signature length.
                    // Only sr25519 signatures are accepted.
                    if attested.attestation.scheme != AttestationScheme::Sr25519 {
                        return InvalidTransaction::BadProof.into();
                    }

                    // Ensure attester is registered.
                    if !RegisteredAttesters::<T>::get(attested.attestation.attester_pubkey) {
                        return InvalidTransaction::BadProof.into();
                    }

                    // Ensure the target intent exists and is not expired.
                    let intent_id = attested.receipt.header.intent_id;
                    let intent = match Intents::<T>::get(intent_id) {
                        Some(i) => i,
                        None => return InvalidTransaction::Stale.into(),
                    };
                    let now = frame_system::Pallet::<T>::block_number();
                    if now > intent.expires_at {
                        return InvalidTransaction::Stale.into();
                    }

                    // Ensure we are still in-progress and the phase matches.
                    let progress = ProgressByIntent::<T>::get(intent_id);
                    if progress.status != SwapStatus::Open {
                        return InvalidTransaction::Stale.into();
                    }
                    if attested.receipt.header.phase != progress.phase {
                        // If the receipt is for a future phase, keep it out of the pool.
                        return InvalidTransaction::Future.into();
                    }

                    // Enforce payload matches the declared domain.
                    let domain = attested.receipt.header.domain_id;
                    let payload_ok = match (domain, &attested.receipt.payload) {
                        (DomainId::Evm, ReceiptPayload::Evm(_)) => true,
                        (DomainId::Svm, ReceiptPayload::Svm(_)) => true,
                        (DomainId::Btc, ReceiptPayload::Btc(_)) => true,
                        (DomainId::X3, ReceiptPayload::X3(_)) => true,
                        _ => false,
                    };
                    if !payload_ok {
                        return InvalidTransaction::BadProof.into();
                    }

                    // Dedupe at the transaction pool level as well.
                    if ReceiptBytes::<T>::get(intent_id, (domain, progress.phase)).is_some() {
                        return InvalidTransaction::Stale.into();
                    }

                    // Verify signature over the canonical receipt bytes.
                    if attested.attestation.signature.len() != 64 {
                        return InvalidTransaction::BadProof.into();
                    }
                    let mut sig_raw = [0u8; 64];
                    sig_raw.copy_from_slice(&attested.attestation.signature[..64]);
                    let sig = sr25519::Signature::from_raw(sig_raw);
                    let pk = sr25519::Public::from_raw(attested.attestation.attester_pubkey);
                    let receipt_bytes = attested.receipt.encode();

                    // Enforce storage-bound size early.
                    if receipt_bytes.len() > (T::MaxReceiptBytes::get() as usize) {
                        return InvalidTransaction::ExhaustsResources.into();
                    }

                    if !sr25519_verify(&sig, &receipt_bytes, &pk) {
                        return InvalidTransaction::BadProof.into();
                    }

                    let receipt_hash = blake2_256(&receipt_bytes);
                    let provides = blake2_256(&(intent_id, domain, progress.phase, receipt_hash).encode());

                    ValidTransaction::with_tag_prefix("ASGA")
                        .priority(100)
                        .longevity(64)
                        .propagate(true)
                        .and_provides(provides)
                        .build()
                }
                _ => InvalidTransaction::Call.into(),
            }
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(10_000)]
        pub fn submit_intent(
            origin: OriginFor<T>,
            intent_id: [u8; 32],
            required_domains: Vec<DomainId>,
            expires_at: BlockNumberFor<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(!Intents::<T>::contains_key(intent_id), Error::<T>::IntentAlreadyExists);

            let bounded_domains: BoundedVec<DomainId, T::MaxDomains> =
                required_domains.try_into().map_err(|_| Error::<T>::TooManyDomains)?;

            let now = frame_system::Pallet::<T>::block_number();
            ensure!(expires_at > now, Error::<T>::IntentExpired);

            Intents::<T>::insert(
                intent_id,
                Intent::<T> { proposer: who.clone(), required_domains: bounded_domains, created_at: now, expires_at },
            );
            ProgressByIntent::<T>::insert(intent_id, Progress::default());

            Self::deposit_event(Event::IntentSubmitted(intent_id, who));
            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(10_000)]
        pub fn register_attester(origin: OriginFor<T>, pubkey: [u8; 32]) -> DispatchResult {
            ensure_root(origin)?;
            RegisteredAttesters::<T>::insert(pubkey, true);
            Self::deposit_event(Event::AttesterRegistered(pubkey));
            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(10_000)]
        pub fn unregister_attester(origin: OriginFor<T>, pubkey: [u8; 32]) -> DispatchResult {
            ensure_root(origin)?;
            RegisteredAttesters::<T>::remove(pubkey);
            Self::deposit_event(Event::AttesterUnregistered(pubkey));
            Ok(())
        }

        /// Unsigned receipt submission path: a validator signs the SCALE bytes of `attested.receipt`.
        #[pallet::call_index(3)]
        #[pallet::weight(10_000)]
        pub fn submit_attested_receipt_unsigned(
            origin: OriginFor<T>,
            attested: AttestedReceipt,
        ) -> DispatchResult {
            ensure_none(origin)?;

            let intent_id = attested.receipt.header.intent_id;
            let intent = Intents::<T>::get(intent_id).ok_or(Error::<T>::IntentNotFound)?;

            let now = frame_system::Pallet::<T>::block_number();
            ensure!(now <= intent.expires_at, Error::<T>::IntentExpired);

            ensure!(
                RegisteredAttesters::<T>::get(attested.attestation.attester_pubkey),
                Error::<T>::AttesterNotRegistered
            );

            Self::verify_attestation(&attested)?;
            Self::accept_receipt(&intent, &attested)?;

            Ok(())
        }

        #[pallet::call_index(4)]
        #[pallet::weight(10_000)]
        pub fn force_revert(origin: OriginFor<T>, intent_id: [u8; 32]) -> DispatchResult {
            ensure_root(origin)?;
            ensure!(Intents::<T>::contains_key(intent_id), Error::<T>::IntentNotFound);

            ProgressByIntent::<T>::mutate(intent_id, |p| {
                p.status = SwapStatus::Reverted;
            });
            Self::deposit_event(Event::Reverted(intent_id));
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        fn verify_attestation(attested: &AttestedReceipt) -> Result<(), DispatchError> {
            ensure!(attested.attestation.scheme == AttestationScheme::Sr25519, Error::<T>::BadAttestation);
            ensure!(attested.attestation.signature.len() == 64, Error::<T>::BadAttestation);

            let mut sig_raw = [0u8; 64];
            sig_raw.copy_from_slice(&attested.attestation.signature[..64]);
            let sig = sr25519::Signature::from_raw(sig_raw);

            let pk = sr25519::Public::from_raw(attested.attestation.attester_pubkey);
            let msg = attested.receipt.encode();

            ensure!(sr25519_verify(&sig, &msg, &pk), Error::<T>::BadAttestation);
            Ok(())
        }

        fn accept_receipt(
            intent: &Intent<T>,
            attested: &AttestedReceipt,
        ) -> Result<(), DispatchError> {
            let intent_id = attested.receipt.header.intent_id;
            let domain = attested.receipt.header.domain_id;
            let phase = attested.receipt.header.phase;

            let mut progress = ProgressByIntent::<T>::get(intent_id);
            ensure!(progress.status == SwapStatus::Open, Error::<T>::AlreadyFinalized);
            ensure!(phase == progress.phase, Error::<T>::InvalidPhase);

            // Enforce payload matches the declared domain.
            let payload_ok = match (domain, &attested.receipt.payload) {
                (DomainId::Evm, ReceiptPayload::Evm(_)) => true,
                (DomainId::Svm, ReceiptPayload::Svm(_)) => true,
                (DomainId::Btc, ReceiptPayload::Btc(_)) => true,
                (DomainId::X3, ReceiptPayload::X3(_)) => true,
                _ => false,
            };
            ensure!(payload_ok, Error::<T>::PayloadDomainMismatch);

            let key = (domain, phase);
            ensure!(ReceiptBytes::<T>::get(intent_id, key).is_none(), Error::<T>::ReceiptAlreadySubmitted);

            let encoded = attested.receipt.encode();
            let receipt_hash = blake2_256(&encoded);
            let bounded: BoundedVec<u8, T::MaxReceiptBytes> =
                encoded.try_into().map_err(|_| Error::<T>::ReceiptTooLarge)?;

            ReceiptBytes::<T>::insert(intent_id, key, bounded);
            Self::deposit_event(Event::ReceiptAccepted(intent_id, domain, phase, receipt_hash));

            // If all required domain receipts are present for this phase, advance.
            if Self::phase_complete(intent_id, intent, progress.phase) {
                progress.phase = match progress.phase {
                    Phase::Lock => Phase::Exec,
                    Phase::Exec => Phase::Final,
                    Phase::Final => {
                        progress.status = SwapStatus::Completed;
                        ProgressByIntent::<T>::insert(intent_id, progress);
                        Self::deposit_event(Event::Completed(intent_id));
                        return Ok(());
                    }
                };

                ProgressByIntent::<T>::insert(intent_id, progress);
                Self::deposit_event(Event::PhaseAdvanced(intent_id, progress.phase));
            }

            Ok(())
        }

        fn phase_complete(
            intent_id: [u8; 32],
            intent: &Intent<T>,
            phase: Phase,
        ) -> bool {
            intent
                .required_domains
                .iter()
                .all(|d| ReceiptBytes::<T>::contains_key(intent_id, (*d, phase)))
        }

        /// Check all ASGA invariants for an intent
        pub fn check_invariants(intent_id: [u8; 32]) -> Vec<InvariantViolation> {
            let mut violations = Vec::new();

            let Some(intent) = Intents::<T>::get(intent_id) else {
                violations.push(InvariantViolation::IntentNotFound);
                return violations;
            };

            let progress = ProgressByIntent::<T>::get(intent_id);
            let now = frame_system::Pallet::<T>::block_number();

            // Invariant 1: Time-bounded execution
            if progress.status == SwapStatus::Open && now > intent.expires_at {
                // Convert block numbers to u64 for storage
                let expires_u64: u64 = intent.expires_at.try_into().unwrap_or(0);
                let current_u64: u64 = now.try_into().unwrap_or(0);
                violations.push(InvariantViolation::DeadlineExceeded {
                    expires_at: expires_u64,
                    current_block: current_u64,
                });
            }

            // Invariant 2: All-or-nothing (check consistency)
            if progress.status == SwapStatus::Completed {
                // Verify all domains have final receipts
                for domain in intent.required_domains.iter() {
                    if !ReceiptBytes::<T>::contains_key(intent_id, (*domain, Phase::Final)) {
                        violations.push(InvariantViolation::MissingFinalReceipt { domain: *domain });
                    }
                }
            }

            // Invariant 3: Phase consistency
            // Can't have exec receipts without lock receipts
            for domain in intent.required_domains.iter() {
                let has_lock = ReceiptBytes::<T>::contains_key(intent_id, (*domain, Phase::Lock));
                let has_exec = ReceiptBytes::<T>::contains_key(intent_id, (*domain, Phase::Exec));
                let has_final = ReceiptBytes::<T>::contains_key(intent_id, (*domain, Phase::Final));

                if has_exec && !has_lock {
                    violations.push(InvariantViolation::PhaseOrderViolation {
                        domain: *domain,
                        has_phase: Phase::Exec,
                        missing_phase: Phase::Lock,
                    });
                }
                if has_final && !has_exec {
                    violations.push(InvariantViolation::PhaseOrderViolation {
                        domain: *domain,
                        has_phase: Phase::Final,
                        missing_phase: Phase::Exec,
                    });
                }
            }

            violations
        }

        /// Enforce timeout invariant - revert swaps that have exceeded deadline
        pub fn enforce_timeout(intent_id: [u8; 32]) -> Result<bool, DispatchError> {
            let intent = Intents::<T>::get(intent_id).ok_or(Error::<T>::IntentNotFound)?;
            let progress = ProgressByIntent::<T>::get(intent_id);
            let now = frame_system::Pallet::<T>::block_number();

            if progress.status == SwapStatus::Open && now > intent.expires_at {
                ProgressByIntent::<T>::mutate(intent_id, |p| {
                    p.status = SwapStatus::Reverted;
                });
                Self::deposit_event(Event::Reverted(intent_id));
                Self::deposit_event(Event::InvariantEnforced(intent_id, InvariantType::Timeout));
                Ok(true)
            } else {
                Ok(false)
            }
        }
    }

    /// Invariant violation types detected by the guardian.
    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum InvariantViolation {
        /// Intent not found
        IntentNotFound,
        /// Deadline exceeded for swap
        DeadlineExceeded {
            expires_at: u64,
            current_block: u64,
        },
        /// Missing final receipt for a domain in a completed swap
        MissingFinalReceipt { domain: DomainId },
        /// Phase order violation (e.g., exec without lock)
        PhaseOrderViolation {
            domain: DomainId,
            has_phase: Phase,
            missing_phase: Phase,
        },
    }

    /// Types of invariants that can be enforced.
    #[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum InvariantType {
        /// Timeout invariant (deadline exceeded)
        Timeout,
        /// All-or-nothing invariant
        AllOrNothing,
        /// Receipt validity
        ReceiptValidity,
        /// Finality safety
        FinalitySafety,
    }
}

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;
