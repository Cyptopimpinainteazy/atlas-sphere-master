//! Types for token launchpad

use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_std::vec::Vec;
use sp_runtime::traits::Zero;

/// Bonding curve type
#[derive(Debug, Clone, Copy, Encode, Decode, TypeInfo, PartialEq, Eq)]
pub enum BondingCurveType {
    Linear,      // price = base + slope * supply
    Exponential, // price = base * e^(exponent * supply)
    Sigmoid,     // S-curve
}

/// Sale status
#[derive(Debug, Clone, Copy, Encode, Decode, TypeInfo, PartialEq, Eq)]
pub enum SaleStatus {
    Active,
    Success,
    Failed,
    Cancelled,
}

/// Token information
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct TokenInfo<AccountId, BlockNumber> {
    pub id: u32,
    pub creator: AccountId,
    pub name: Vec<u8>,
    pub symbol: Vec<u8>,
    pub initial_supply: u128,
    pub total_raised: u128,
    pub curve_type: BondingCurveType,
    pub min_purchase: u128,
    pub max_purchase: u128,
    pub hard_cap: u128,
    pub created_at: BlockNumber,
    pub end_at: BlockNumber,
    pub status: SaleStatus,
    pub team_members: Vec<AccountId>,
}

/// Vesting schedule
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct VestingSchedule<BlockNumber> {
    pub total: u128,
    pub claimed: u128,
    pub start_block: BlockNumber,
    pub cliff_block: BlockNumber,
    pub end_block: BlockNumber,
}
