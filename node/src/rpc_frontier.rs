//! Frontier RPC wiring stub
//!
//! This module provides optional wiring for Frontier JSON-RPC endpoints.
//! When `feature = "frontier"` is enabled for the node crate, this module
//! will create and merge additional Ethereum-compatible RPC handlers. These
//! should be replaced (or extended) with the `fc-rpc`/`fp-rpc` modules once
//! the Frontier version compatibility is resolved.

use crate::rpc::{AccountId, AssetId, Balance, Block, Nonce};
use hex;
use jsonrpsee::core::RpcResult;
use jsonrpsee::RpcModule;
use sc_client_api::BlockBackend;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use std::sync::Arc;

/// Create a minimal frontier compatible module stub.
/// This currently adds a placeholder `eth_call` stub that delegates to
/// `EthCompatApi`/`AtlasKernelApi` where appropriate. Replace this with
/// fc-rpc integration when the runtime and Frontier versions are compatible.
pub fn create_frontier_stub<C, P>(
    client: Arc<C>,
    _pool: Arc<P>,
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
where
    C: Send
        + Sync
        + 'static
        + ProvideRuntimeApi<Block>
        + HeaderBackend<Block>
        + BlockBackend<Block>,
    C::Api: crate::rpc::AtlasKernelRuntimeApi<Block, AccountId, Balance, AssetId>,
{
    let mut module = RpcModule::new(());

    // Minimal eth_getBalance RPC that maps an EVM address (hex) to canonical balance via runtime API.
    let rpc_client = client.clone();
    module.register_method("eth_getBalance", move |params, _| {
        let address_hex: String = params.one()?;
        let addr_str = address_hex.strip_prefix("0x").unwrap_or(&address_hex);
        let bytes =
            hex::decode(addr_str).map_err(|e| jsonrpsee::types::Error::Custom(e.to_string()))?;

        let api = rpc_client.runtime_api();
        let at = rpc_client.info().best_hash;
        let asset_id: AssetId = 0u32; // Native asset id
        let balance: Balance = api
            .get_evm_balance(at, bytes, asset_id)
            .map_err(|e| jsonrpsee::types::Error::Custom(format!("Runtime error: {:?}", e)))?
            .unwrap_or_default();

        let hex_balance = format!("0x{:x}", balance);
        Ok(hex_balance)
    })?;
    Ok(module)
}
