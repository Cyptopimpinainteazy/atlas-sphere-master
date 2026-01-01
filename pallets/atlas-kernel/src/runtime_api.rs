// Runtime API trait is defined in this pallet's lib.rs using sp_api::decl_runtime_apis!
// The macro automatically adds Block as the first generic parameter.
// 
// Declaration: AtlasKernelRuntimeApi<AccountId, Balance, AssetId>
// Expands to: AtlasKernelRuntimeApi<Block, AccountId, Balance, AssetId>
// 
// Implementation Location:
//   - File: runtime/src/lib.rs
//   - Block: impl_runtime_apis! { impl pallet_atlas_kernel::AtlasKernelRuntimeApi<Block, ...> for Runtime { ... } }
//   - Lines: ~395-420
//   - Methods: get_canonical_balance, get_asset_metadata, is_authorized, get_authorized_accounts, get_authorities
//
// RPC Consumption Location:
//   - File: node/src/rpc.rs
//   - Struct: AtlasKernelRpc<C, B>
//   - Trait Bound: C::Api: AtlasKernelRuntimeApi<Block, AccountId, Balance, AssetId>
//   - Lines: ~71, ~141
//   - Exposed Methods: atlasKernel_getCanonicalBalance, atlasKernel_getAssetMetadata, atlasKernel_isAuthorized, atlasKernel_getAuthorizedAccounts, atlasKernel_getAuthorities
//
// For easy discovery, re-export the trait from this module:
pub use crate::AtlasKernelRuntimeApi;
