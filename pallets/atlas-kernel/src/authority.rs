/// Authority Set Management for Atlas Sphere
///
/// This module manages the validator set, authority changes, and session rotation.
/// It integrates with Substrate's session pallet to handle consensus participants.
///
/// Authority changes follow a two-phase commit pattern:
/// 1. Changes are scheduled via `schedule_authority_change`
/// 2. Changes are enacted at the next session boundary via `enact_authority_change`
///
/// This prevents mid-session disruptions and ensures consensus stability.
use frame_support::pallet_prelude::*;
use sp_std::vec::Vec;

// Note: This module defines authority management logic but storage lives in the main pallet
// to avoid circular dependencies. The pallet implements these functions as helper methods.

/// Authority set change type
#[derive(Clone, RuntimeDebug, Encode, Decode, PartialEq, Eq, TypeInfo)]
pub enum AuthorityChange<AccountId> {
    /// New authority added
    Added(AccountId),
    /// Authority removed
    Removed(AccountId),
    /// Complete authority set changed
    SetChanged(Vec<AccountId>),
}

/// Result type for authority operations
pub type AuthorityResult = Result<(), DispatchError>;

/// Authority management errors
#[derive(Clone, RuntimeDebug, PartialEq, Eq)]
pub enum AuthorityError {
    /// Authority already exists in the set
    AlreadyAuthority,
    /// Authority not found in the set
    NotAnAuthority,
    /// Would violate minimum authorities constraint
    BelowMinimumAuthorities,
    /// Would exceed maximum authorities constraint
    ExceedsMaximumAuthorities,
    /// No pending changes to enact
    NoPendingChanges,
    /// Authority set is empty
    EmptyAuthoritySet,
}

impl From<AuthorityError> for &'static str {
    fn from(err: AuthorityError) -> &'static str {
        match err {
            AuthorityError::AlreadyAuthority => "Authority already in set",
            AuthorityError::NotAnAuthority => "Not an authority",
            AuthorityError::BelowMinimumAuthorities => "Below minimum authorities",
            AuthorityError::ExceedsMaximumAuthorities => "Exceeds maximum authorities",
            AuthorityError::NoPendingChanges => "No pending changes",
            AuthorityError::EmptyAuthoritySet => "Empty authority set",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authority_change_serialization() {
        // Test that AuthorityChange can be encoded/decoded
        let change = AuthorityChange::Added(42u64);
        let encoded = change.encode();
        let decoded = AuthorityChange::<u64>::decode(&mut &encoded[..]).unwrap();
        assert_eq!(change, decoded);
    }

    #[test]
    fn authority_error_messages() {
        // Test error message conversion
        let err: &'static str = AuthorityError::AlreadyAuthority.into();
        assert_eq!(err, "Authority already in set");

        let err: &'static str = AuthorityError::BelowMinimumAuthorities.into();
        assert_eq!(err, "Below minimum authorities");
    }
}
