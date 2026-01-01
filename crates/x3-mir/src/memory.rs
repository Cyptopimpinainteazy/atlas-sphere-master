//! Memory model abstraction for MIR.
//! Four models provided: Register, Stack, Heap, GlobalStorage.
//!
//! Used by optimizer passes to decide safety of reordering, elision, and hoisting.

use std::fmt;

/// Which memory model an access targets.
///
/// Used to classify Load/Store operations and guide optimizer decision-making.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MemoryModel {
    /// Ephemeral virtual register space (no aliasing, no side-effects).
    /// Loads/stores here are pure register moves in lowering.
    Register,

    /// Function-local stack slot. May require spill/restore semantics.
    /// No cross-function aliasing; within-function aliasing possible.
    Stack,

    /// Local heap allocated data. May alias; optimizer must be conservative.
    Heap,

    /// On-chain persistent storage / global state. Side-effecting, persistent.
    /// Must not be elided or reordered across atomic boundaries.
    GlobalStorage,
}

impl MemoryModel {
    /// Does this model represent persistent on-chain storage with side effects?
    pub fn is_persistent(self) -> bool {
        matches!(self, MemoryModel::GlobalStorage)
    }

    /// Conservative test: can this model alias with arbitrary other addresses?
    ///
    /// - Register: no aliasing (each reg is distinct)
    /// - Stack: local aliasing only (within function)
    /// - Heap: aliasing possible (different ptrs may point to overlapping data)
    /// - GlobalStorage: yes (persistent storage is global)
    pub fn may_alias(self) -> bool {
        match self {
            MemoryModel::Register => false,
            MemoryModel::Stack => false, // local slots don't alias across functions
            MemoryModel::Heap => true,
            MemoryModel::GlobalStorage => true,
        }
    }

    /// Has observable side effects (stores visible to other threads / blocks)?
    pub fn has_side_effects(self) -> bool {
        match self {
            MemoryModel::GlobalStorage => true,
            // Stores to Register/Stack are local; Heap stores might be visible
            // but we treat conservatively (see may_alias)
            _ => false,
        }
    }

    /// Memory model lowering preference (lower first -> higher priority).
    /// Hint for code layout and register allocation.
    pub fn lowering_preference(self) -> u8 {
        match self {
            MemoryModel::Register => 0,
            MemoryModel::Stack => 10,
            MemoryModel::Heap => 20,
            MemoryModel::GlobalStorage => 30,
        }
    }
}

impl fmt::Display for MemoryModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryModel::Register => write!(f, "Register"),
            MemoryModel::Stack => write!(f, "Stack"),
            MemoryModel::Heap => write!(f, "Heap"),
            MemoryModel::GlobalStorage => write!(f, "GlobalStorage"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_model_display() {
        assert_eq!(MemoryModel::Register.to_string(), "Register");
        assert_eq!(MemoryModel::Stack.to_string(), "Stack");
        assert_eq!(MemoryModel::Heap.to_string(), "Heap");
        assert_eq!(MemoryModel::GlobalStorage.to_string(), "GlobalStorage");
    }

    #[test]
    fn memory_model_persistence() {
        assert!(!MemoryModel::Register.is_persistent());
        assert!(!MemoryModel::Stack.is_persistent());
        assert!(!MemoryModel::Heap.is_persistent());
        assert!(MemoryModel::GlobalStorage.is_persistent());
    }

    #[test]
    fn memory_model_aliasing() {
        assert!(!MemoryModel::Register.may_alias());
        assert!(!MemoryModel::Stack.may_alias());
        assert!(MemoryModel::Heap.may_alias());
        assert!(MemoryModel::GlobalStorage.may_alias());
    }

    #[test]
    fn memory_model_side_effects() {
        assert!(!MemoryModel::Register.has_side_effects());
        assert!(!MemoryModel::Stack.has_side_effects());
        assert!(!MemoryModel::Heap.has_side_effects());
        assert!(MemoryModel::GlobalStorage.has_side_effects());
    }

    #[test]
    fn memory_model_lowering_order() {
        let mut models = vec![
            MemoryModel::GlobalStorage,
            MemoryModel::Register,
            MemoryModel::Heap,
            MemoryModel::Stack,
        ];
        models.sort_by_key(|m| m.lowering_preference());
        assert_eq!(
            models,
            vec![
                MemoryModel::Register,
                MemoryModel::Stack,
                MemoryModel::Heap,
                MemoryModel::GlobalStorage,
            ]
        );
    }
}
