//! Atlas Sphere RPC extensions
//!
//! Provides JSON-RPC endpoints for interacting with the Atlas Sphere node.
//! Includes standard Substrate RPC methods plus Atlas Kernel custom methods.

use std::sync::Arc;

use atlas_sphere_runtime::{opaque::Block, AccountId, Balance, Index};
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockchainError, HeaderBackend, HeaderMetadata};

pub use sc_rpc_api::DenyUnsafe;

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
    C: HeaderBackend<Block> + 'static,
    C: Send + Sync + 'static,
{
    fn not_implemented(method: &'static str) -> jsonrpsee::types::ErrorObjectOwned {
        jsonrpsee::types::ErrorObjectOwned::owned(
            -32004,
            format!("{method} is not implemented on this node build"),
            None::<()>,
        )
    }

    let mut module = jsonrpsee::RpcModule::new(client);

    // Get canonical balance for an account and asset
    module.register_method("atlasKernel_getCanonicalBalance", |params, _client, _| {
        let _: (String, u32) = params.parse()?;
        Err::<String, jsonrpsee::types::ErrorObjectOwned>(not_implemented(
            "atlasKernel_getCanonicalBalance",
        ))
    })?;

    // Check if account is authorized for Comit submission
    module.register_method("atlasKernel_isAuthorized", |params, _client, _| {
        let _: String = params.one()?;
        Err::<bool, jsonrpsee::types::ErrorObjectOwned>(not_implemented("atlasKernel_isAuthorized"))
    })?;

    // Get asset metadata
    module.register_method("atlasKernel_getAssetMetadata", |params, _client, _| {
        let _: u32 = params.one()?;
        Err::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(not_implemented(
            "atlasKernel_getAssetMetadata",
        ))
    })?;

    // Get Comit execution receipt
    module.register_method("atlasKernel_getComitReceipt", |params, _client, _| {
        let _: String = params.one()?;
        Err::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(not_implemented(
            "atlasKernel_getComitReceipt",
        ))
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
        let _: String = params.one()?;
        Err::<u64, jsonrpsee::types::ErrorObjectOwned>(not_implemented("atlasKernel_getNonce"))
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

    module.register_method("indexer_getBlock", |params, _client, _| {
        let _: u64 = params.one()?;
        Err::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(not_implemented(
            "indexer_getBlock",
        ))
    })?;

    // Get events by pallet and event name
    module.register_method("indexer_getEvents", |params, _client, _| {
        let _: (String, String) = params.parse()?;
        Err::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(not_implemented(
            "indexer_getEvents",
        ))
    })?;

    // Get extrinsics for a block
    module.register_method("indexer_getExtrinsics", |params, _client, _| {
        let _: u64 = params.one()?;
        Err::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(not_implemented(
            "indexer_getExtrinsics",
        ))
    })?;

    // Get last indexed block number
    module.register_method("indexer_getLastIndexedBlock", |_params, _client, _| {
        Err::<u64, jsonrpsee::types::ErrorObjectOwned>(not_implemented(
            "indexer_getLastIndexedBlock",
        ))
    })?;

    // ============================================================================
    // SWARM EXECUTOR RPC METHODS (CB-006: Agent autonomy)
    // ============================================================================

    // Submit a job for agent execution
    module.register_method("swarmExecutor_submitJob", |params, _client, _| {
        let _: (String, String, u8) = params.parse()?;
        Err::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(not_implemented(
            "swarmExecutor_submitJob",
        ))
    })?;

    // Get job status
    module.register_method("swarmExecutor_getJob", |params, _client, _| {
        let _: u64 = params.one()?;
        Err::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(not_implemented(
            "swarmExecutor_getJob",
        ))
    })?;

    // List pending jobs
    module.register_method("swarmExecutor_listPendingJobs", |_params, _client, _| {
        Err::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(not_implemented(
            "swarmExecutor_listPendingJobs",
        ))
    })?;

    // Register an agent
    module.register_method("swarmExecutor_registerAgent", |params, _client, _| {
        let _: (String, String, String) = params.parse()?;
        Err::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(not_implemented(
            "swarmExecutor_registerAgent",
        ))
    })?;

    // Get agent info
    module.register_method("swarmExecutor_getAgent", |params, _client, _| {
        let _: String = params.one()?;
        Err::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(not_implemented(
            "swarmExecutor_getAgent",
        ))
    })?;

    // Create agent proposal for consensus
    module.register_method("swarmExecutor_createProposal", |params, _client, _| {
        let _: (String, String, String) = params.parse()?;
        Err::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(not_implemented(
            "swarmExecutor_createProposal",
        ))
    })?;

    // Vote on proposal
    module.register_method("swarmExecutor_voteOnProposal", |params, _client, _| {
        let _: (u64, String, String) = params.parse()?;
        Err::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(not_implemented(
            "swarmExecutor_voteOnProposal",
        ))
    })?;

    // Get swarm statistics
    module.register_method("swarmExecutor_getStats", |_params, _client, _| {
        Err::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(not_implemented(
            "swarmExecutor_getStats",
        ))
    })?;

    // ============================================================================
    // PHASE 5: FLASH LOAN RPC METHODS
    // ============================================================================

    // Request a flash loan
    module.register_method("flashloan_requestFlashLoan", |params, _client, _| {
        let _: (u32, String) = params.parse()?;
        Err::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(not_implemented(
            "flashloan_requestFlashLoan",
        ))
    })?;

    // Repay a flash loan
    module.register_method("flashloan_repayFlashLoan", |params, _client, _| {
        let _: (u64, u32) = params.parse()?;
        Err::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(not_implemented(
            "flashloan_repayFlashLoan",
        ))
    })?;

    // Get flash loan details
    module.register_method("flashloan_getLoan", |params, _client, _| {
        let _: u64 = params.one()?;
        Err::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(not_implemented(
            "flashloan_getLoan",
        ))
    })?;

    // Get flash loan statistics
    module.register_method("flashloan_getStats", |_params, _client, _| {
        Err::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(not_implemented(
            "flashloan_getStats",
        ))
    })?;

    // Get borrower's loans
    module.register_method("flashloan_getBorrowerLoans", |params, _client, _| {
        let _: String = params.one()?;
        Err::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(not_implemented(
            "flashloan_getBorrowerLoans",
        ))
    })?;

    // ============================================================================
    // PHASE 5: ORACLE RPC METHODS
    // ============================================================================

    // Get current price for a feed
    module.register_method("oracle_getPrice", |params, _client, _| {
        let _: u32 = params.one()?;
        Err::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(not_implemented(
            "oracle_getPrice",
        ))
    })?;

    // Get price history for a feed
    module.register_method("oracle_getPriceHistory", |params, _client, _| {
        let _: (u32, u32) = params.parse()?;
        Err::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(not_implemented(
            "oracle_getPriceHistory",
        ))
    })?;

    // Get volatility for a feed
    module.register_method("oracle_getVolatility", |params, _client, _| {
        let _: u32 = params.one()?;
        Err::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(not_implemented(
            "oracle_getVolatility",
        ))
    })?;

    // Aggregate prices from multiple sources
    module.register_method("oracle_aggregatePrices", |params, _client, _| {
        let _: (Vec<u32>, String) = params.parse()?;
        Err::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(not_implemented(
            "oracle_aggregatePrices",
        ))
    })?;

    // Check if price has deviated significantly
    module.register_method("oracle_checkDeviation", |params, _client, _| {
        let _: (u32, String, String, u32) = params.parse()?;
        Err::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(not_implemented(
            "oracle_checkDeviation",
        ))
    })?;

    // Check if price is stale
    module.register_method("oracle_isPriceStale", |params, _client, _| {
        let _: u32 = params.one()?;
        Err::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(not_implemented(
            "oracle_isPriceStale",
        ))
    })?;

    // Get consensus price from multiple oracles
    module.register_method("oracle_getConsensusPrice", |params, _client, _| {
        let _: Vec<u32> = params.one()?;
        Err::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(not_implemented(
            "oracle_getConsensusPrice",
        ))
    })?;

    // Get oracle statistics
    module.register_method("oracle_getStats", |_params, _client, _| {
        Err::<serde_json::Value, jsonrpsee::types::ErrorObjectOwned>(not_implemented(
            "oracle_getStats",
        ))
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
