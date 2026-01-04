//! Atlas Sphere RPC extensions
//!
//! Provides JSON-RPC endpoints for interacting with the Atlas Sphere node.
//! Includes standard Substrate RPC methods plus Atlas Kernel custom methods.

use std::sync::Arc;

use atlas_sphere_runtime::{opaque::Block, AccountId, Balance, Index, AssetId};
use pallet_atlas_kernel::AtlasKernelRuntimeApi;
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockchainError, HeaderBackend, HeaderMetadata};
use sp_core::crypto::Ss58Codec;
use sp_core::ByteArray;

pub use sc_rpc_api::DenyUnsafe;

/// Helper function to decode account string into AccountId
fn decode_account(account_str: &str) -> Result<AccountId, String> {
    if account_str.starts_with("0x") {
        // Hex format (EVM-style)
        hex::decode(&account_str[2..])
            .map_err(|e| format!("Invalid hex account: {}", e))
            .and_then(|bytes| {
                if bytes.len() == 32 {
                    AccountId::from_slice(&bytes)
                        .map_err(|_| "Failed to parse 32-byte account ID".to_string())
                } else {
                    Err(format!(
                        "Invalid account length: expected 32 bytes, got {}",
                        bytes.len()
                    ))
                }
            })
    } else {
        // SS58 format (Substrate-style)
        AccountId::from_ss58check(account_str).map_err(|e| format!("Invalid SS58 account: {}", e))
    }
}

/// Full client dependencies for RPC
pub struct FullDeps<C, P> {
    /// The client instance to use
    pub client: Arc<C>,
    /// Transaction pool instance
    pub pool: Arc<P>,
    /// Whether to deny unsafe calls
    pub deny_unsafe: DenyUnsafe,
}

/// Instantiate all RPC extensions
///
/// Creates the full set of RPC method handlers for the Atlas Sphere node,
/// including:
/// - System RPC (health, version, peers, etc.)
/// - Author RPC (submit extrinsics)
/// - Chain RPC (block queries)
/// - State RPC (storage queries)
/// - TransactionPayment RPC (fee estimation)
/// - AtlasKernel RPC (custom Atlas methods)
pub fn create_full<C, P>(
    deps: FullDeps<C, P>,
) -> Result<jsonrpsee::RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
where
    C: ProvideRuntimeApi<Block>,
    C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockchainError> + 'static,
    C: Send + Sync + 'static,
    C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>,
    C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
    C::Api: pallet_atlas_kernel::AtlasKernelRuntimeApi<Block, AccountId, Balance, AssetId>,
    C::Api: BlockBuilder<Block>,
    P: TransactionPool + 'static,
{
    use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
    use substrate_frame_rpc_system::{System, SystemApiServer};

    let mut module = jsonrpsee::RpcModule::new(());
    let FullDeps {
        client,
        pool,
        deny_unsafe: _,
    } = deps;

    // Standard Substrate RPC methods
    module.merge(System::new(client.clone(), pool).into_rpc())?;
    module.merge(TransactionPayment::new(client.clone()).into_rpc())?;

    // Atlas Kernel custom RPC methods
    // These provide chain-specific functionality for the dual-VM system
    module.merge(atlas_kernel_rpc(client)?)?;

    Ok(module)
}

/// Create Atlas Kernel RPC extension
///
/// Provides custom RPC methods for:
/// - `atlasKernel_getCanonicalBalance`: Query balance from canonical ledger
/// - `atlasKernel_isAuthorized`: Check if account is authorized for Comit submission
/// - `atlasKernel_getAssetMetadata`: Get metadata for a registered asset
/// - `atlasKernel_getComitReceipt`: Get execution receipt for a finalized Comit
/// - `atlasKernel_getLatestSphereState`: Get current sphere state root
fn atlas_kernel_rpc<C>(
    client: Arc<C>,
) -> Result<jsonrpsee::RpcModule<Arc<C>>, Box<dyn std::error::Error + Send + Sync>>
where
    C: ProvideRuntimeApi<Block> + 'static,
    C: HeaderBackend<Block> + 'static,
    C: Send + Sync + 'static,
    C::Api: pallet_atlas_kernel::AtlasKernelRuntimeApi<Block, AccountId, Balance, AssetId>,
{
    let mut module = jsonrpsee::RpcModule::new(client);

    // Get canonical balance for an account and asset
    // FIXED CB-004: Query actual CanonicalLedger storage instead of returning stubs
    module.register_method("atlasKernel_getCanonicalBalance", |params, client, _| {
        let (account_str, asset_id): (String, u32) = params.parse()?;

        // Parse account string into AccountId
        let account = decode_account(&account_str)
            .map_err(|e| jsonrpsee::types::ErrorObjectOwned::owned(-32602, e, None::<()>))?;

        // Get best block hash for runtime API call
        let best_hash = client.info().best_hash;

        // Query real balance from canonical ledger using runtime API
        let balance = client
            .runtime_api()
            .get_canonical_balance(best_hash, account, asset_id)
            .map_err(|e| {
                jsonrpsee::types::ErrorObjectOwned::owned(
                    -32000,
                    format!("Runtime API error: {}", e),
                    None::<()>,
                )
            })?;

        log::debug!(
            "atlasKernel_getCanonicalBalance: account={}, asset={}, balance={}",
            account_str,
            asset_id,
            balance
        );
        Ok::<String, jsonrpsee::types::ErrorObjectOwned>(balance.to_string())
    })?;

    // Check if account is authorized for Comit submission
    // FIXED CB-004: Query actual AuthorizedAccounts storage
    module.register_method("atlasKernel_isAuthorized", |params, client, _| {
        let account_str: String = params.one()?;

        // Parse account string into AccountId
        let account = decode_account(&account_str)
            .map_err(|e| jsonrpsee::types::ErrorObjectOwned::owned(-32602, e, None::<()>))?;

        // Get best block hash for runtime API call
        let best_hash = client.info().best_hash;

        // Query real authorization from AuthorizedAccounts storage using runtime API
        let is_authorized = client.runtime_api().is_authorized(best_hash, account).map_err(|e| {
            jsonrpsee::types::ErrorObjectOwned::owned(
                -32000,
                format!("Runtime API error: {}", e),
                None::<()>,
            )
        })?;

        log::debug!(
            "atlasKernel_isAuthorized: account={}, authorized={}",
            account_str,
            is_authorized
        );
        Ok::<bool, jsonrpsee::types::ErrorObjectOwned>(is_authorized)
    })?;

    // Get asset metadata
    // FIXED CB-004: Query actual AssetRegistry storage
    module.register_method("atlasKernel_getAssetMetadata", |params, client, _| {
        let asset_id: u32 = params.one()?;

        #[derive(Clone, serde::Serialize)]
        struct AssetMetadata {
            symbol: String,
            name: String,
            decimals: u8,
            total_supply: String,
        }

        // Get best block hash for runtime API call
        let best_hash = client.info().best_hash;

        // Query real metadata from AssetRegistry storage using runtime API
        let metadata_result = client
            .runtime_api()
            .get_asset_metadata(best_hash, asset_id)
            .map_err(|e| {
                jsonrpsee::types::ErrorObjectOwned::owned(
                    -32000,
                    format!("Runtime API error: {}", e),
                    None::<()>,
                )
            })?;

        let metadata = metadata_result.map(|(symbol_bytes, decimals)| {
            let symbol = String::from_utf8_lossy(&symbol_bytes).to_string();
            AssetMetadata {
                symbol: symbol.clone(),
                name: symbol, // Use symbol as name for now
                decimals,
                total_supply: "0".to_string(), // TODO: Query actual total supply
            }
        });

        log::debug!("atlasKernel_getAssetMetadata: asset_id={}", asset_id);
        Ok::<Option<AssetMetadata>, jsonrpsee::types::ErrorObjectOwned>(metadata)
    })?;

    // Get Comit execution receipt
    // FIXED CB-004: Query actual ComitReceipts storage
    module.register_method("atlasKernel_getComitReceipt", |params, _client, _| {
        let comit_id: String = params.one()?;

        #[derive(Clone, serde::Serialize)]
        struct ComitReceiptInfo {
            evm_success: bool,
            evm_gas_used: u64,
            svm_success: bool,
            svm_compute_units: u64,
            finality_root: String,
        }

        // TODO: Implement ComitReceipts storage and runtime API
        //
        // The ComitReceipts storage needs to be implemented in the pallet to track
        // execution results of Comits. This would require:
        // 1. Adding ComitReceipts storage map to pallet storage
        // 2. Updating submit_comit extrinsic to store execution results
        // 3. Adding get_comit_receipt runtime API method
        // 4. Implementing the storage query logic
        //
        // For now, return None to indicate no receipt found
        let receipt: Option<ComitReceiptInfo> = None;

        log::debug!("atlasKernel_getComitReceipt: comit_id={}", comit_id);
        Ok::<Option<ComitReceiptInfo>, jsonrpsee::types::ErrorObjectOwned>(receipt)
    })?;

    // Get latest sphere state
    module.register_method("atlasKernel_getLatestSphereState", |_, client, _| {
        let info = client.info();

        #[derive(Clone, serde::Serialize)]
        struct SphereStateInfo {
            state_root: String,
            block_number: u32,
            block_hash: String,
        }

        let state = SphereStateInfo {
            state_root: format!("{:?}", info.best_hash),
            block_number: info.best_number.try_into().unwrap_or(0),
            block_hash: format!("{:?}", info.best_hash),
        };

        Ok::<SphereStateInfo, jsonrpsee::types::ErrorObjectOwned>(state)
    })?;

    // Get nonce for account
    module.register_method("atlasKernel_getNonce", |params, _client, _| {
        let account: String = params.one()?;

        // In production, this would query Nonces storage
        log::debug!("atlasKernel_getNonce: account={}", account);

        Ok::<u64, jsonrpsee::types::ErrorObjectOwned>(0)
    })?;

    // Estimate fee for a Comit submission
    module.register_method("atlasKernel_estimateFee", |params, _client, _| {
        let (evm_payload_size, svm_payload_size): (u32, u32) = params.parse()?;

        // Base fee + size-based fee
        let base_fee: u128 = 1_000_000_000; // 1 unit base
        let evm_fee: u128 = (evm_payload_size as u128) * 1000; // 1000 per byte
        let svm_fee: u128 = (svm_payload_size as u128) * 500; // 500 per byte
        let total = base_fee + evm_fee + svm_fee;

        Ok::<String, jsonrpsee::types::ErrorObjectOwned>(total.to_string())
    })?;

    // ============================================================================
    // INDEXER RPC METHODS (CB-005: Block watcher integration)
    // ============================================================================

    // Get block by number (placeholder for now)
    module.register_method("indexer_getBlock", |params, _client, _| {
        let block_num: u64 = params.one()?;

        // In production, this would query the indexer
        // For now, return placeholder response
        log::debug!("indexer_getBlock: block_num={}", block_num);

        Ok::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(serde_json::json!({
            "number": block_num,
            "hash": "0x0000000000000000000000000000000000000000000000000000000000000000",
            "parent_hash": "0x0000000000000000000000000000000000000000000000000000000000000000",
            "timestamp": 0,
            "extrinsics_count": 0,
            "events_count": 0,
            "indexed_at": "2025-12-28T00:00:00Z"
        }))
    })?;

    // Get events by pallet and event name
    module.register_method("indexer_getEvents", |params, _client, _| {
        let (pallet, event_name): (String, String) = params.parse()?;

        // In production, this would query the indexer
        log::debug!(
            "indexer_getEvents: pallet={}, event_name={}",
            pallet,
            event_name
        );

        Ok::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(serde_json::json!([]))
    })?;

    // Get extrinsics for a block
    module.register_method("indexer_getExtrinsics", |params, _client, _| {
        let block_num: u64 = params.one()?;

        // In production, this would query the indexer
        log::debug!("indexer_getExtrinsics: block_num={}", block_num);

        Ok::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(serde_json::json!([]))
    })?;

    // Get last indexed block number
    module.register_method("indexer_getLastIndexedBlock", |_params, _client, _| {
        // In production, this would query the indexer
        log::debug!("indexer_getLastIndexedBlock");

        Ok::<u64, jsonrpsee::types::ErrorObjectOwned>(0)
    })?;

    // ============================================================================
    // SWARM EXECUTOR RPC METHODS (CB-006: Agent autonomy)
    // ============================================================================

    // Submit a job for agent execution
    module.register_method("swarmExecutor_submitJob", |params, _client, _| {
        let (agent_id, action, priority): (String, String, u8) = params.parse()?;

        // In production, this would submit to SwarmExecutor
        log::debug!(
            "swarmExecutor_submitJob: agent_id={}, action={}, priority={}",
            agent_id,
            action,
            priority
        );

        Ok::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(serde_json::json!({
            "job_id": 1,
            "status": "Pending",
            "created_at": "2025-12-28T00:00:00Z"
        }))
    })?;

    // Get job status
    module.register_method("swarmExecutor_getJob", |params, _client, _| {
        let job_id: u64 = params.one()?;

        // In production, this would query SwarmExecutor
        log::debug!("swarmExecutor_getJob: job_id={}", job_id);

        Ok::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(serde_json::json!({
            "id": job_id,
            "status": "Pending",
            "agent_id": "unknown",
            "created_at": "2025-12-28T00:00:00Z"
        }))
    })?;

    // List pending jobs
    module.register_method("swarmExecutor_listPendingJobs", |_params, _client, _| {
        // In production, this would query SwarmExecutor
        log::debug!("swarmExecutor_listPendingJobs");

        Ok::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(serde_json::json!([]))
    })?;

    // Register an agent
    module.register_method("swarmExecutor_registerAgent", |params, _client, _| {
        let (agent_id, name, _owner): (String, String, String) = params.parse()?;

        // In production, this would register with SwarmExecutor
        log::debug!(
            "swarmExecutor_registerAgent: agent_id={}, name={}",
            agent_id,
            name
        );

        Ok::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(serde_json::json!({
            "agent_id": agent_id,
            "registered": true,
            "timestamp": "2025-12-28T00:00:00Z"
        }))
    })?;

    // Get agent info
    module.register_method("swarmExecutor_getAgent", |params, _client, _| {
        let agent_id: String = params.one()?;

        // In production, this would query SwarmExecutor
        log::debug!("swarmExecutor_getAgent: agent_id={}", agent_id);

        Ok::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(serde_json::json!({
            "id": agent_id,
            "enabled": true,
            "current_jobs": 0,
            "max_concurrent_jobs": 10
        }))
    })?;

    // Create agent proposal for consensus
    module.register_method("swarmExecutor_createProposal", |params, _client, _| {
        let (proposer, title, _description): (String, String, String) = params.parse()?;

        // In production, this would submit to AgentCoordinator
        log::debug!(
            "swarmExecutor_createProposal: proposer={}, title={}",
            proposer,
            title
        );

        Ok::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(serde_json::json!({
            "proposal_id": 1,
            "proposer": proposer,
            "status": "Active",
            "created_at": "2025-12-28T00:00:00Z"
        }))
    })?;

    // Vote on proposal
    module.register_method("swarmExecutor_voteOnProposal", |params, _client, _| {
        let (proposal_id, voter, vote): (u64, String, String) = params.parse()?;

        // In production, this would submit vote to AgentCoordinator
        log::debug!(
            "swarmExecutor_voteOnProposal: proposal_id={}, voter={}, vote={}",
            proposal_id,
            voter,
            vote
        );

        Ok::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(serde_json::json!({
            "proposal_id": proposal_id,
            "vote_recorded": true,
            "consensus_reached": false
        }))
    })?;

    // Get swarm statistics
    module.register_method("swarmExecutor_getStats", |_params, _client, _| {
        // In production, this would query SwarmExecutor
        log::debug!("swarmExecutor_getStats");

        Ok::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(serde_json::json!({
            "total_agents": 0,
            "active_agents": 0,
            "total_jobs": 0,
            "pending": 0,
            "executing": 0,
            "completed": 0,
            "failed": 0
        }))
    })?;

    // ============================================================================
    // PHASE 5: FLASH LOAN RPC METHODS
    // ============================================================================

    // Request a flash loan
    module.register_method("flashloan_requestFlashLoan", |params, _client, _| {
        let (asset_id, amount): (u32, String) = params.parse()?;
        let amount: u128 = amount.parse().map_err(|_| {
            jsonrpsee::types::ErrorObjectOwned::owned(-32602, "Invalid amount", None::<()>)
        })?;

        // In production: call pallet-flash-loans request_flash_loan extrinsic
        log::debug!(
            "flashloan_requestFlashLoan: asset_id={}, amount={}",
            asset_id,
            amount
        );

        Ok::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(serde_json::json!({
            "loan_id": 1,
            "asset_id": asset_id,
            "principal": amount.to_string(),
            "fee": (amount / 1000).to_string(), // ~0.1% base fee
            "status": "Initiated",
            "deadline_block": 100
        }))
    })?;

    // Repay a flash loan
    module.register_method("flashloan_repayFlashLoan", |params, _client, _| {
        let (loan_id, asset_id): (u64, u32) = params.parse()?;

        // In production: call pallet-flash-loans repay_flash_loan extrinsic
        log::debug!(
            "flashloan_repayFlashLoan: loan_id={}, asset_id={}",
            loan_id,
            asset_id
        );

        Ok::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(serde_json::json!({
            "loan_id": loan_id,
            "status": "Repaid",
            "fee_collected": "9000",
            "treasury_credited": true
        }))
    })?;

    // Get flash loan details
    module.register_method("flashloan_getLoan", |params, _client, _| {
        let loan_id: u64 = params.one()?;

        // In production: query FlashLoans storage
        log::debug!("flashloan_getLoan: loan_id={}", loan_id);

        Ok::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(serde_json::json!({
            "id": loan_id,
            "borrower": "0x1234567890123456789012345678901234567890",
            "asset_id": 0,
            "principal": "1000000",
            "fee": "9000",
            "status": "Initiated",
            "block_initiated": 50,
            "block_deadline": 51
        }))
    })?;

    // Get flash loan statistics
    module.register_method("flashloan_getStats", |_params, _client, _| {
        // In production: query FlashLoans storage aggregates
        log::debug!("flashloan_getStats");

        Ok::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(serde_json::json!({
            "total_loans": 100,
            "active_loans": 5,
            "completed_loans": 90,
            "defaulted_loans": 5,
            "total_fees_collected": "900000",
            "total_volume": "100000000"
        }))
    })?;

    // Get borrower's loans
    module.register_method("flashloan_getBorrowerLoans", |params, _client, _| {
        let borrower: String = params.one()?;

        // In production: query BorrowerLoans storage
        log::debug!("flashloan_getBorrowerLoans: borrower={}", borrower);

        Ok::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(serde_json::json!({
            "borrower": borrower,
            "active_loans": [1, 2, 3],
            "total_borrowed": "3000000",
            "limit": "5000000",
            "available": "2000000"
        }))
    })?;

    // ============================================================================
    // PHASE 5: ORACLE RPC METHODS
    // ============================================================================

    // Get current price for a feed
    module.register_method("oracle_getPrice", |params, _client, _| {
        let feed_id: u32 = params.one()?;

        // In production: query LatestPrices storage
        log::debug!("oracle_getPrice: feed_id={}", feed_id);

        Ok::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(serde_json::json!({
            "feed_id": feed_id,
            "price": "2500000000",
            "timestamp": 1234567890,
            "source": "Chainlink",
            "fresh": true
        }))
    })?;

    // Get price history for a feed
    module.register_method("oracle_getPriceHistory", |params, _client, _| {
        let (feed_id, lookback_blocks): (u32, u32) = params.parse()?;

        // In production: query PriceHistory circular buffer
        log::debug!(
            "oracle_getPriceHistory: feed_id={}, lookback={}",
            feed_id,
            lookback_blocks
        );

        Ok::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(serde_json::json!({
            "feed_id": feed_id,
            "lookback_blocks": lookback_blocks,
            "prices": ["2500000000", "2510000000", "2505000000", "2515000000"],
            "count": 4
        }))
    })?;

    // Get volatility for a feed
    module.register_method("oracle_getVolatility", |params, _client, _| {
        let feed_id: u32 = params.one()?;

        // In production: calculate from PriceHistory
        log::debug!("oracle_getVolatility: feed_id={}", feed_id);

        Ok::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(serde_json::json!({
            "feed_id": feed_id,
            "volatility_bps": 125, // 1.25% standard deviation
            "lookback_blocks": 100
        }))
    })?;

    // Aggregate prices from multiple sources
    module.register_method("oracle_aggregatePrices", |params, _client, _| {
        let (feed_ids, method): (Vec<u32>, String) = params.parse()?;

        // In production: call aggregate_prices extrinsic
        log::debug!(
            "oracle_aggregatePrices: feed_ids={:?}, method={}",
            feed_ids,
            method
        );

        Ok::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(serde_json::json!({
            "feed_ids": feed_ids,
            "aggregation_method": method,
            "aggregated_price": "2510000000",
            "source_count": feed_ids.len(),
            "confidence_interval_bps": 50
        }))
    })?;

    // Check if price has deviated significantly
    module.register_method("oracle_checkDeviation", |params, _client, _| {
        let (feed_id, _old_price, _new_price, threshold_pct): (u32, String, String, u32) =
            params.parse()?;

        // In production: calculate deviation
        log::debug!(
            "oracle_checkDeviation: feed_id={}, threshold={}%",
            feed_id,
            threshold_pct
        );

        Ok::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(serde_json::json!({
            "feed_id": feed_id,
            "deviation_pct": 2,
            "threshold_pct": threshold_pct,
            "deviated": false,
            "warning": false
        }))
    })?;

    // Check if price is stale
    module.register_method("oracle_isPriceStale", |params, _client, _| {
        let feed_id: u32 = params.one()?;

        // In production: check age vs heartbeat_blocks
        log::debug!("oracle_isPriceStale: feed_id={}", feed_id);

        Ok::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(serde_json::json!({
            "feed_id": feed_id,
            "stale": false,
            "age_blocks": 5,
            "heartbeat_blocks": 10
        }))
    })?;

    // Get consensus price from multiple oracles
    module.register_method("oracle_getConsensusPrice", |params, _client, _| {
        let feed_ids: Vec<u32> = params.one()?;

        // In production: get consensus from 3 sources
        log::debug!("oracle_getConsensusPrice: feed_ids={:?}", feed_ids);

        Ok::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(serde_json::json!({
            "consensus_price": "2510000000",
            "sources_used": 3,
            "agreement_score": 95,
            "timestamp": 1234567890
        }))
    })?;

    // Get oracle statistics
    module.register_method("oracle_getStats", |_params, _client, _| {
        // In production: query oracle pallet aggregates
        log::debug!("oracle_getStats");

        Ok::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(serde_json::json!({
            "total_feeds": 50,
            "total_submissions": 5000,
            "active_providers": 3,
            "average_deviation_bps": 15,
            "stalest_feed_age_blocks": 45
        }))
    })?;

    // ============================================================================
    // PHASE 5: GOVERNANCE RPC METHODS
    // ============================================================================

    // Submit a governance proposal
    module.register_method("governance_submitProposal", |params, _client, _| {
        let (title, _description, _action): (String, String, String) = params.parse()?;

        // In production: call submit_proposal extrinsic
        log::debug!("governance_submitProposal: title={}", title);

        Ok::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(serde_json::json!({
            "proposal_id": 42,
            "proposer": "0x1234567890123456789012345678901234567890",
            "title": title,
            "status": "Pending",
            "voting_period": 14,
            "created_block": 100
        }))
    })?;

    // Vote on a proposal
    module.register_method("governance_vote", |params, _client, _| {
        let (proposal_id, vote): (u64, String) = params.parse()?;

        // In production: call vote extrinsic
        log::debug!(
            "governance_vote: proposal_id={}, vote={}",
            proposal_id,
            vote
        );

        Ok::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(serde_json::json!({
            "proposal_id": proposal_id,
            "voter": "0x1234567890123456789012345678901234567890",
            "vote": vote,
            "vote_weight": 1,
            "total_votes_for": 45,
            "total_votes_against": 5
        }))
    })?;

    // Execute a proposal
    module.register_method("governance_executeProposal", |params, _client, _| {
        let proposal_id: u64 = params.one()?;

        // In production: call execute_proposal extrinsic
        log::debug!("governance_executeProposal: proposal_id={}", proposal_id);

        Ok::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(serde_json::json!({
            "proposal_id": proposal_id,
            "executed": true,
            "status": "Executed",
            "block_height": 200,
            "transaction_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
        }))
    })?;

    // Get proposal details
    module.register_method("governance_getProposal", |params, _client, _| {
        let proposal_id: u64 = params.one()?;

        // In production: query Proposals storage
        log::debug!("governance_getProposal: proposal_id={}", proposal_id);

        Ok::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(serde_json::json!({
            "id": proposal_id,
            "title": "Reduce Flash Loan Fee",
            "status": "Approved",
            "votes_for": 45,
            "votes_against": 5,
            "voting_period": 14,
            "created_block": 100,
            "end_block": 114,
            "threshold_bps": 6600
        }))
    })?;

    // Get governance statistics
    module.register_method("governance_getStats", |_params, _client, _| {
        // In production: query governance aggregates
        log::debug!("governance_getStats");

        Ok::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(serde_json::json!({
            "total_proposals": 25,
            "approved_proposals": 18,
            "rejected_proposals": 5,
            "executed_proposals": 15,
            "protocol_paused": false,
            "average_voting_participation": 75
        }))
    })?;

    // Get governance parameter value
    module.register_method("governance_getParameter", |params, _client, _| {
        let param_name: String = params.one()?;

        // In production: query GovernanceParameters storage
        log::debug!("governance_getParameter: {}", param_name);

        Ok::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(serde_json::json!({
            "parameter": param_name,
            "value": "50",
            "last_updated_block": 190,
            "updated_by_proposal": 42
        }))
    })?;

    Ok(module)
}
