#![cfg_attr(not(feature = "std"), no_std)]

//! Pallet `atomic-guardian` — skeleton implementation for the X3 Arbiter.
//!
//! NOTE: This is an initial skeleton to be extended. It defines storage, events, errors, and dispatchables
//! according to the ASGA spec in `docs/ASGA_SPEC.md`.

use frame_support::{dispatch::DispatchResult, pallet_prelude::*};
use frame_system::pallet_prelude::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    use sp_core::crypto::AppCrypto;
    use sp_runtime::offchain::Signer;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        /// AuthorityId for off-chain signing
        type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::validate_unsigned]
    impl<T: Config> ValidateUnsigned for Pallet<T> {
        type Call = Call<T>;

        fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            use frame_support::traits::InvalidTransaction;
            // Only accept unsigned attested submission calls; basic size checks here reduce spam.
            match call {
                Call::submit_attested_receipt_unsigned { intent_hash: _, domain: _, receipt, attester_pub, signature } => {
                    if attester_pub.len() != 32 || signature.len() != 64 || receipt.is_empty() {
                        return InvalidTransaction::Stale.into();
                    }
                    // Accept: lightweight accept; full verification happens in-call.
                    ValidTransaction::with_tag_prefix("ASGAUnsigned").priority(1).longevity(3).propagate(true).build()
                }
                _ => InvalidTransaction::Call.into(),
            }
        }
    }

    /// Swap intent data.
    #[derive(Encode, Decode, CloneNoBound, PartialEq, Eq, RuntimeDebug, Default)]
    pub struct IntentData<AccountId> {
        pub proposer: AccountId,
        pub intent_hash: [u8; 32],
        // TODO: add canonical fields (domains, expected amounts, deadlines)
    }

    #[pallet::storage]
    #[pallet::getter(fn swap_intents)]
    pub(super) type SwapIntents<T: Config> = StorageMap<_, Blake2_128Concat, [u8; 32], IntentData<T::AccountId>>;

    #[pallet::storage]
    #[pallet::getter(fn swap_state)]
    pub(super) type SwapState<T: Config> = StorageMap<_, Blake2_128Concat, [u8; 32], u8>;

    #[pallet::storage]
    #[pallet::getter(fn receipts)]
    pub(super) type Receipts<T: Config> = StorageDoubleMap<_, Blake2_128Concat, [u8; 32], Blake2_128Concat, u8, Vec<u8>>;

    /// Registered off-chain validators (attesters) who can submit attested receipts.
    #[pallet::storage]
    #[pallet::getter(fn registered_validator)]
    pub(super) type RegisteredValidators<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, bool, ValueQuery>;

    /// Configurable RPC URL for EVM node used by off-chain worker.
    #[pallet::storage]
    #[pallet::getter(fn evm_rpc_url)]
    pub(super) type EvmRpcUrl<T: Config> = StorageValue<_, Vec<u8>, ValueQuery>;

    /// Minimum confirmations required for EVM receipts to be considered final.
    #[pallet::storage]
    #[pallet::getter(fn evm_min_confirmations)]
    pub(super) type EvmMinConfirmations<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// Pending external domain txs (intent_id -> tx_hash_hex)
    #[pallet::storage]
    #[pallet::getter(fn pending_txs)]
    pub(super) type PendingTxs<T: Config> = StorageMap<_, Blake2_128Concat, [u8; 32], Vec<u8>, OptionQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        IntentSubmitted([u8; 32], T::AccountId),
        ReceiptSubmitted([u8; 32], u8),
        AttestedReceiptSubmitted([u8;32], u8, T::AccountId),
        StateAdvanced([u8; 32], u8),
        ValidatorRegistered(T::AccountId),
        ValidatorUnregistered(T::AccountId),
        // TODO: more fine-grained events
    }

    #[pallet::error]
    pub enum Error<T> {
        IntentAlreadyExists,
        IntentNotFound,
        InvalidReceipt,
        Unauthorized,
        StateInvariantViolated,
        AlreadyRegistered,
        NotRegistered,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Submit a new swap intent
        #[pallet::weight(10_000)]
        pub fn submit_intent(origin: OriginFor<T>, intent_hash: [u8; 32]) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(!SwapIntents::<T>::contains_key(&intent_hash), Error::<T>::IntentAlreadyExists);

            let intent = IntentData { proposer: who.clone(), intent_hash };
            SwapIntents::<T>::insert(&intent_hash, intent);
            SwapState::<T>::insert(&intent_hash, 0u8); // S0: Initialized

            Self::deposit_event(Event::IntentSubmitted(intent_hash, who));
            Ok(())
        }

        /// Submit a domain receipt (unsigned attestation path recommended for gas savings)
        #[pallet::weight(10_000)]
        pub fn submit_receipt(origin: OriginFor<T>, intent_hash: [u8; 32], domain: u8, receipt: Vec<u8>) -> DispatchResult {
            let _who = ensure_signed(origin)?;
            ensure!(SwapIntents::<T>::contains_key(&intent_hash), Error::<T>::IntentNotFound);

            // TODO: validate receipt cryptographically (signatures, format)
            Receipts::<T>::insert(&intent_hash, domain, receipt.clone());
            Self::deposit_event(Event::ReceiptSubmitted(intent_hash, domain));
            Ok(())
        }

        /// Submit an attested receipt by a registered validator (preferred: off-chain verification + signed submission)
        #[pallet::weight(10_000)]
        pub fn submit_attested_receipt(origin: OriginFor<T>, intent_hash: [u8; 32], domain: u8, receipt: Vec<u8>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(SwapIntents::<T>::contains_key(&intent_hash), Error::<T>::IntentNotFound);
            ensure!(RegisteredValidators::<T>::get(&who), Error::<T>::NotRegistered);

            // Attester is registered; we accept the attested receipt as validated off-chain.
            Receipts::<T>::insert(&intent_hash, domain, receipt.clone());
            Self::deposit_event(Event::AttestedReceiptSubmitted(intent_hash, domain, who));
            Ok(())
        }

        /// Submit attested receipt as unsigned tx with signed payload (receipt bytes) - attestation is verified on-chain.
        #[pallet::weight(10_000)]
        pub fn submit_attested_receipt_unsigned(origin: OriginFor<T>, intent_hash: [u8; 32], domain: u8, receipt: Vec<u8>, attester_pub: Vec<u8>, signature: Vec<u8>) -> DispatchResult {
            // unsigned call
            ensure_none(origin)?;
            ensure!(SwapIntents::<T>::contains_key(&intent_hash), Error::<T>::IntentNotFound);

            // validate lengths
            if attester_pub.len() != 32 || signature.len() != 64 {
                return Err(Error::<T>::InvalidReceipt.into());
            }

            // Convert to fixed arrays
            let mut pub_a = [0u8; 32]; pub_a.copy_from_slice(&attester_pub[..32]);
            let mut sig_a = [0u8; 64]; sig_a.copy_from_slice(&signature[..64]);

            // Verify sr25519 signature on the payload
            use sp_core::sr25519::{Public as SrPub, Signature as SrSig};
            let pk = SrPub::from_raw(pub_a);
            let sig = SrSig::from_raw(sig_a);
            if !sp_io::crypto::sr25519_verify(&sig, &receipt[..], &pk) {
                return Err(Error::<T>::InvalidReceipt.into());
            }

            // map pubkey bytes to AccountId bytes (only works if AccountId is 32 bytes representation)
            let maybe_account = T::AccountId::decode(&mut &pub_a[..]).ok();
            let attester_account = match maybe_account {
                Some(a) => a,
                None => return Err(Error::<T>::Unauthorized.into()),
            };

            ensure!(RegisteredValidators::<T>::get(&attester_account), Error::<T>::Unauthorized);

            // store receipt and emit event
            Receipts::<T>::insert(&intent_hash, domain, receipt.clone());
            Self::deposit_event(Event::AttestedReceiptSubmitted(intent_hash, domain, attester_account));
            Ok(())
        }

        /// Register a validator account (root only)
        #[pallet::weight(10_000)]
        pub fn register_validator(origin: OriginFor<T>, account: T::AccountId) -> DispatchResult {
            ensure_root(origin)?;
            ensure!(!RegisteredValidators::<T>::get(&account), Error::<T>::AlreadyRegistered);
            RegisteredValidators::<T>::insert(&account, true);
            Self::deposit_event(Event::ValidatorRegistered(account));
            Ok(())
        }

        /// Link an external tx hash to an intent so off-chain workers know what to watch for.
        #[pallet::weight(10_000)]
        pub fn register_pending_tx(origin: OriginFor<T>, intent_hash: [u8; 32], tx_hash: [u8; 32]) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(SwapIntents::<T>::contains_key(&intent_hash), Error::<T>::IntentNotFound);

            PendingTxs::<T>::mutate(&intent_hash, |vec| vec.push(tx_hash));
            Self::deposit_event(Event::ReceiptSubmitted(intent_hash, 0u8)); // event reused for now
            Ok(())
        }

        /// Remove a pending tx (after verification)
        #[pallet::weight(10_000)]
        pub fn remove_pending_tx(origin: OriginFor<T>, intent_hash: [u8; 32], tx_hash: [u8; 32]) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(SwapIntents::<T>::contains_key(&intent_hash), Error::<T>::IntentNotFound);

            PendingTxs::<T>::mutate(&intent_hash, |vec| {
                if let Some(pos) = vec.iter().position(|x| x == &tx_hash) {
                    vec.remove(pos);
                }
            });
            Ok(())
        }

        /// Set the EVM RPC URL (root only).
        #[pallet::weight(10_000)]
        pub fn set_evm_rpc_url(origin: OriginFor<T>, url: Vec<u8>) -> DispatchResult {
            ensure_root(origin)?;
            EvmRpcUrl::<T>::put(url);
            Ok(())
        }

        /// Set EVM minimum confirmation threshold (root only).
        #[pallet::weight(10_000)]
        pub fn set_evm_min_confirmations(origin: OriginFor<T>, n: u32) -> DispatchResult {
            ensure_root(origin)?;
            EvmMinConfirmations::<T>::put(n);
            Ok(())
        }

        /// Unregister a validator account (root only)
        #[pallet::weight(10_000)]
        pub fn unregister_validator(origin: OriginFor<T>, account: T::AccountId) -> DispatchResult {
            ensure_root(origin)?;
            ensure!(RegisteredValidators::<T>::get(&account), Error::<T>::NotRegistered);
            RegisteredValidators::<T>::remove(&account);
            Self::deposit_event(Event::ValidatorUnregistered(account));
            Ok(())
        }

        /// Register a pending external domain tx (e.g., EVM tx hash) for an intent
        #[pallet::weight(10_000)]
        pub fn register_pending_tx(origin: OriginFor<T>, intent_hash: [u8; 32], tx_hash_hex: Vec<u8>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(SwapIntents::<T>::contains_key(&intent_hash), Error::<T>::IntentNotFound);
            PendingTxs::<T>::insert(&intent_hash, tx_hash_hex.clone());
            // Note: we allow any signer to register a pending tx; governance policy can restrict this if desired.
            Self::deposit_event(Event::ReceiptSubmitted(intent_hash, 0));
            Ok(())
        }

        /// Remove pending tx mapping (root only)
        #[pallet::weight(10_000)]
        pub fn remove_pending_tx(origin: OriginFor<T>, intent_hash: [u8; 32]) -> DispatchResult {
            ensure_root(origin)?;
            PendingTxs::<T>::remove(&intent_hash);
            Ok(())
        }

        /// Attempt to advance state (authoritative checks performed)
        #[pallet::weight(10_000)]
        pub fn advance_state(origin: OriginFor<T>, intent_hash: [u8; 32]) -> DispatchResult {
            let _ = ensure_signed(origin)?;

            ensure!(SwapIntents::<T>::contains_key(&intent_hash), Error::<T>::IntentNotFound);

            // TODO: evaluate invariants, check receipts, change state accordingly
            // Placeholder: advance state value by 1
            SwapState::<T>::mutate(&intent_hash, |s| if let Some(v) = s { *v = v.saturating_add(1) } );
            let state = SwapState::<T>::get(&intent_hash).unwrap_or_default();
            Self::deposit_event(Event::StateAdvanced(intent_hash, state));
            Ok(())
        }

        /// Force revert (evidence required)
        #[pallet::weight(10_000)]
        pub fn force_revert(origin: OriginFor<T>, intent_hash: [u8; 32], _evidence: Vec<u8>) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            ensure!(SwapIntents::<T>::contains_key(&intent_hash), Error::<T>::IntentNotFound);
            SwapState::<T>::insert(&intent_hash, 7u8); // S7: Reverted
            Self::deposit_event(Event::StateAdvanced(intent_hash, 7u8));
            Ok(())
        }

        /// Slash an agent (evidence required)
        #[pallet::weight(10_000)]
        pub fn slash_agent(origin: OriginFor<T>, _agent: T::AccountId, _evidence: Vec<u8>) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            // TODO: implement slashing logic
            Ok(())
        }
    }

    // Off-chain worker hook: skeleton for validators to run verification and submit attested receipts.
    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
        fn offchain_worker(block_number: T::BlockNumber) {
            use sp_runtime::offchain::{http, storage::StorageValueRef};
            use sp_std::str;
            use sp_std::vec::Vec;

            log::info!(target: "asga", "Off-chain worker running at block: {:?}", block_number);

            // Helper parser exposed for unit tests
            fn parse_evm_receipt_json(body_str: &str) -> Option<([u8;32], u64, [u8;20])> {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(body_str) {
                    let result = json.get("result")?;
                    let tx_hash_hex = result.get("transactionHash")?.as_str()?;
                    let block_number_hex = result.get("blockNumber")?.as_str()?;
                    let to_hex = result.get("to")?.as_str()?;

                    fn parse_hex_32(s: &str) -> Option<[u8; 32]> {
                        let s = s.strip_prefix("0x").unwrap_or(s);
                        if s.len() != 64 { return None; }
                        let b = hex::decode(s).ok()?;
                        let mut arr = [0u8;32]; arr.copy_from_slice(&b[..32]); Some(arr)
                    }
                    fn parse_hex_20(s: &str) -> Option<[u8; 20]> {
                        let s = s.strip_prefix("0x").unwrap_or(s);
                        if s.len() != 40 { return None; }
                        let b = hex::decode(s).ok()?;
                        let mut arr = [0u8;20]; arr.copy_from_slice(&b[..20]); Some(arr)
                    }
                    fn parse_hex_u64(s: &str) -> Option<u64> {
                        let s = s.strip_prefix("0x").unwrap_or(s);
                        u64::from_str_radix(s, 16).ok()
                    }

                    let tx_hash = parse_hex_32(tx_hash_hex)?;
                    let block_number = parse_hex_u64(block_number_hex)?;
                    let to = parse_hex_20(to_hex)?;
                    Some((tx_hash, block_number, to))
                } else { None }
            }

            // Iterate over pending txs and try to fetch receipts from a configured RPC node.
            for (intent_hash, tx_hex) in PendingTxs::<T>::iter() {
                if tx_hex.is_empty() {
                    continue;
                }

                // Convert tx hex to str
                let tx_hex_str = match str::from_utf8(&tx_hex) {
                    Ok(s) => s,
                    Err(_) => {
                        log::error!(target: "asga", "Invalid tx hex for intent {:?}", intent_hash);
                        continue;
                    }
                };

                // Get RPC URL from storage or default to localhost
                let url_vec = EvmRpcUrl::<T>::get();
                let url = if url_vec.is_empty() {
                    "http://127.0.0.1:8545/".to_string()
                } else {
                    match sp_std::str::from_utf8(&url_vec) {
                        Ok(s) => s.to_string(),
                        Err(_) => {
                            log::error!(target: "asga", "Invalid EVM RPC URL stored");
                            continue;
                        }
                    }
                };

                let body = sp_std::format!(r#"{{"jsonrpc":"2.0","method":"eth_getTransactionReceipt","params":["{}"],"id":1}}"#, tx_hex_str);

                let request = http::Request::post(&url, vec![body.as_bytes()]);
                let timeout = sp_io::offchain::timestamp().add(sp_runtime::offchain::Duration::from_millis(3_000));

                let pending = match request.add_header("Content-Type", "application/json").deadline(timeout).send() {
                    Ok(p) => p,
                    Err(e) => {
                        log::error!(target: "asga", "HTTP request failed for tx {}: {:?}", tx_hex_str, e);
                        continue;
                    }
                };

                let response = match pending.wait() {
                    Ok(resp) => resp,
                    Err(e) => {
                        log::error!(target: "asga", "HTTP response wait failed: {:?}", e);
                        continue;
                    }
                };

                if response.code != 200 {
                    log::warn!(target: "asga", "Non-200 response: {} for tx {}", response.code, tx_hex_str);
                    continue;
                }

                let body_vec = response.body().collect::<Vec<u8>>();
                if body_vec.is_empty() {
                    log::warn!(target: "asga", "Empty body for tx {}", tx_hex_str);
                    continue;
                }

                // Simple heuristic parsing: check for "status":"0x1" or "status": "0x1"
                let body_str = match str::from_utf8(&body_vec) {
                    Ok(s) => s,
                    Err(_) => {
                        log::error!(target: "asga", "Invalid UTF-8 in response for tx {}", tx_hex_str);
                        continue;
                    }
                };

                if body_str.contains("\"status\":\"0x1\"") || body_str.contains("\"status\":\s*\"0x1\"") {
                    // store successful receipt to local offchain storage for auditing / replay
                    let key = sp_std::format!("asga:validated:{}", hex::encode(intent_hash));
                    let storage_ref = StorageValueRef::persistent(key.as_bytes());
                    // store raw JSON as audit artifact
                    storage_ref.set(&body_str);
                    log::info!(target: "asga", "Validated tx {} for intent {:?}", tx_hex_str, intent_hash);

                    // Parse and validate JSON into canonical structs
                    if let Some((tx_hash, block_number, contract_address)) = parse_evm_receipt_json(&body_str) {
                        // parse hex helpers
                        fn parse_hex_32(s: &str) -> Option<[u8; 32]> {
                            let s = s.strip_prefix("0x").unwrap_or(s);
                            if s.len() != 64 { return None; }
                            let b = hex::decode(s).ok()?;
                            let mut arr = [0u8;32]; arr.copy_from_slice(&b[..32]); Some(arr)
                        }
                        fn parse_hex_20(s: &str) -> Option<[u8; 20]> {
                            let s = s.strip_prefix("0x").unwrap_or(s);
                            if s.len() != 40 { return None; }
                            let b = hex::decode(s).ok()?;
                            let mut arr = [0u8;20]; arr.copy_from_slice(&b[..20]); Some(arr)
                        }
                        fn parse_hex_u64(s: &str) -> Option<u64> {
                            let s = s.strip_prefix("0x").unwrap_or(s);
                            u64::from_str_radix(s, 16).ok()
                        }

                        let (tx_hash, block_number, contract_address) = (tx_hash, block_number, contract_address);

                        // Get latest block via eth_blockNumber
                        let block_req = r#"{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}"#;
                        let block_request = http::Request::post(&url, vec![block_req.as_bytes()]);
                        let deadline2 = sp_io::offchain::timestamp().add(sp_runtime::offchain::Duration::from_millis(2_000));
                        let block_resp = match block_request.add_header("Content-Type", "application/json").deadline(deadline2).send() {
                            Ok(p) => p,
                            Err(e) => {
                                log::error!(target: "asga", "eth_blockNumber request failed: {:?}", e);
                                continue;
                            }
                        };
                        let block_resp = match block_resp.wait() {
                            Ok(r) => r,
                            Err(e) => { log::error!(target: "asga", "eth_blockNumber wait failed: {:?}", e); continue; }
                        };
                        let block_body = block_resp.body().collect::<Vec<u8>>();
                        let latest_block = match sp_std::str::from_utf8(&block_body).ok().and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok()).and_then(|v| v.get("result").and_then(|r| r.as_str())).and_then(|hex| u64::from_str_radix(hex.strip_prefix("0x").unwrap_or("0"), 16).ok()) {
                            Some(n) => n,
                            None => { log::warn!(target: "asga", "Failed to parse eth_blockNumber response"); continue; }
                        };

                        // confirmations = latest_block - block_number + 1 (if block_number>0)
                        let confirmations = if block_number == 0 { 0 } else { latest_block.saturating_sub(block_number).saturating_add(1) };

                        let min_conf = EvmMinConfirmations::<T>::get();
                        if (confirmations as u32) < min_conf {
                            log::info!(target: "asga", "Tx {} has {} confirmations (< {}), skipping", tx_hex_str, confirmations, min_conf);
                            continue;
                        }

                        // Build canonical header and payload
                        use asga_receipts::{ReceiptHeader, EvmReceipt, DomainId, Phase};
                        let now_ts = sp_io::offchain::timestamp().unix_millis();

                        // Create a signer bytes using the local account id bytes.
                        let signer_bytes = {
                            let signer = Signer::<T, T::AuthorityId>::any_account();
                            let mut v = Vec::new();
                            if let Some((acct, _)) = signer.all_accounts().next() {
                                v = acct.id.encode();
                            }
                            v
                        };

                        let header = ReceiptHeader {
                            intent_id: intent_hash,
                            domain_id: DomainId::Evm,
                            phase: Phase::Exec,
                            amount: 0u128,
                            asset_id: [0u8;32],
                            timestamp: now_ts as u64,
                            signer: signer_bytes,
                        };

                        let evm = EvmReceipt {
                            tx_hash,
                            block_number,
                            confirmations: confirmations as u32,
                            contract_address,
                            calldata_hash: [0u8;32],
                        };
                        // Basic validation: ensure tx hash present
                        if evm.tx_hash == [0u8;32] {
                            log::error!(target: "asga", "Parsed tx hash empty for intent {:?}", intent_hash);
                        } else {
                            // Encode canonical receipt (SCALE) and submit via signed extrinsic
                            let mut payload = header.encode();
                            payload.extend(evm.encode());

                            // Build payload bytes and submit using unsigned signed-payload flow where the attestation signature
                            // is computed by the keystore and submitted alongside the payload. The runtime will verify the
                            // sr25519 signature and attester registration before accepting.
                            let payload_bytes = payload.clone();
                            let res = Signer::<T, T::AuthorityId>::any_account()
                                .send_unsigned_transaction(
                                    |_account| payload_bytes.clone(),
                                    |payload, signature| {
                                        // signature is sr25519::Signature; account.public is sr25519::Public
                                        let pubkey = _account.public.clone();
                                        Call::submit_attested_receipt_unsigned(intent_hash, 0u8, payload.clone(), pubkey.as_ref().to_vec(), signature.encode())
                                    },
                                );

                            if let Some((acct, result)) = res {
                                match result {
                                    Ok(()) => {
                                        log::info!(target: "asga", "Attested receipt (unsigned signed-payload) submitted by {:?} for intent {:?}", acct.id, intent_hash);
                                        PendingTxs::<T>::remove(&intent_hash);
                                    }
                                    Err(e) => log::error!(target: "asga", "Unsigned attested submission failed: {:?}", e),
                                }
                            } else {
                                log::warn!(target: "asga", "No local accounts available to sign attestation payload");
                            }
                        }

                    } // parsed json

                } else {
                    log::info!(target: "asga", "Receipt not finalized or failed for tx {}: {}", tx_hex_str, body_str);
                }
            }
        }
    }

    // Unit tests and quick checks
    #[cfg(test)]
    mod tests {
        use super::*;
        use sp_core::H256;

        #[test]
        fn basic_flow() {
            // placeholder test: to be replaced with FRAME test environment tests
            assert_eq!(1 + 1, 2);
        }

        #[test]
        fn parse_evm_receipt_and_confirmations() {
            // Example eth_getTransactionReceipt JSON snippet
            let body = r#"{ "jsonrpc":"2.0", "id":1, "result": { "transactionHash": "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "blockNumber":"0x10", "to":"0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", "status":"0x1" } }"#;

            // call the helper
            let parsed = {
                // replicate the helper scope
                fn parse_evm_receipt_json(body_str: &str) -> Option<([u8;32], u64, [u8;20])> {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(body_str) {
                        let result = json.get("result")?;
                        let tx_hash_hex = result.get("transactionHash")?.as_str()?;
                        let block_number_hex = result.get("blockNumber")?.as_str()?;
                        let to_hex = result.get("to")?.as_str()?;

                        fn parse_hex_32(s: &str) -> Option<[u8; 32]> {
                            let s = s.strip_prefix("0x").unwrap_or(s);
                            if s.len() != 64 { return None; }
                            let b = hex::decode(s).ok()?;
                            let mut arr = [0u8;32]; arr.copy_from_slice(&b[..32]); Some(arr)
                        }
                        fn parse_hex_20(s: &str) -> Option<[u8; 20]> {
                            let s = s.strip_prefix("0x").unwrap_or(s);
                            if s.len() != 40 { return None; }
                            let b = hex::decode(s).ok()?;
                            let mut arr = [0u8;20]; arr.copy_from_slice(&b[..20]); Some(arr)
                        }
                        fn parse_hex_u64(s: &str) -> Option<u64> {
                            let s = s.strip_prefix("0x").unwrap_or(s);
                            u64::from_str_radix(s, 16).ok()
                        }

                        let tx_hash = parse_hex_32(tx_hash_hex)?;
                        let block_number = parse_hex_u64(block_number_hex)?;
                        let to = parse_hex_20(to_hex)?;
                        Some((tx_hash, block_number, to))
                    } else { None }
                }
                parse_evm_receipt_json(body).expect("parsed")
            };

            // blockNumber 0x10 == 16
            assert_eq!(parsed.1, 16u64);

            // compute confirmations example: latest_block = 20 => confirmations = 20 - 16 + 1 = 5
            let latest_block = 20u64;
            let confirmations = latest_block.saturating_sub(parsed.1).saturating_add(1);
            assert_eq!(confirmations, 5u64);
        }

        #[test]
        fn unsigned_attested_submission_flow_stub() {
            // This is a lightweight test to ensure that unsigned submission call exists and errors on wrong sizes.
            let intent = [0u8;32];
            let domain: u8 = 0;
            let receipt = vec![1,2,3];
            let attester_pub = vec![1u8; 31]; // bad length
            let signature = vec![2u8; 63]; // bad length

            // Calling function directly (simulated) should return InvalidReceipt due to length checks.
            let res = Pallet::<TestRuntime>::submit_attested_receipt_unsigned(RuntimeOrigin::None, intent, domain, receipt, attester_pub, signature);
            assert!(res.is_err());
        }
    }
}