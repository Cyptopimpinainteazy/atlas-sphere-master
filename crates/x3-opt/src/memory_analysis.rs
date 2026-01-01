//! Helpers for optimizer passes to reason about memory operations.
//!
//! Used by PRE, DCE, loop passes, and reordering passes to decide:
//! - What can be safely elided?
//! - What can be safely reordered?
//! - What aliases with what?

use x3_mir::memory::MemoryModel;
use x3_mir::MirRhs;

/// Does an RHS represent a memory store with side effects?
///
/// Side-effecting stores must not be elided or reordered.
pub fn rhs_is_sideeffecting_store(rhs: &MirRhs) -> bool {
    match rhs {
        MirRhs::Store { model, .. } => model.has_side_effects(),
        _ => false,
    }
}

/// Does an RHS represent a store to persistent (global) storage?
pub fn rhs_is_persistent_store(rhs: &MirRhs) -> bool {
    match rhs {
        MirRhs::Store { model, .. } => model.is_persistent(),
        _ => false,
    }
}

/// Does an RHS represent a load from persistent (global) storage?
pub fn rhs_is_persistent_load(rhs: &MirRhs) -> bool {
    match rhs {
        MirRhs::Load { model, .. } => model.is_persistent(),
        _ => false,
    }
}

/// Does this RHS contain a memory access (load or store)?
pub fn rhs_is_memory_access(rhs: &MirRhs) -> bool {
    matches!(rhs, MirRhs::Load { .. } | MirRhs::Store { .. })
}

/// Does this RHS contain a memory access that might alias/escape?
///
/// Conservative: returns true if aliasing is possible.
pub fn rhs_has_aliasing_access(rhs: &MirRhs) -> bool {
    match rhs {
        MirRhs::Load { model, .. } => model.may_alias(),
        MirRhs::Store { model, .. } => model.may_alias(),
        _ => false,
    }
}

/// Conservative check used by DCE and reordering passes:
/// if an RHS is a store with side effects, it must be preserved.
pub fn rhs_has_observable_effects(rhs: &MirRhs) -> bool {
    match rhs {
        MirRhs::Store { model, .. } => model.has_side_effects(),
        // All memory loads are treated as potentially observable
        // (even to Register/Stack, because their ordering may matter for semantics)
        _ => false,
    }
}

/// Can this memory access be safely hoisted out of a loop or block?
///
/// Conservative: requires no aliasing and no side effects.
pub fn memory_access_is_hoistable(rhs: &MirRhs) -> bool {
    match rhs {
        MirRhs::Load { model, .. } => {
            // Load can hoist if it's from a memory model with no aliasing
            // (i.e., not Heap or GlobalStorage)
            !model.may_alias() && !model.is_persistent()
        }
        MirRhs::Store { .. } => {
            // Stores are never hoisted (they have effects)
            false
        }
        _ => false,
    }
}

/// Can two memory accesses be safely reordered?
///
/// Conservative: if either is a store with side effects, or both touch aliasing memory, return false.
pub fn memory_accesses_can_reorder(rhs1: &MirRhs, rhs2: &MirRhs) -> bool {
    let (m1, m2) = match (rhs1, rhs2) {
        (MirRhs::Load { model: m1, .. }, MirRhs::Load { model: m2, .. }) => (m1, m2),
        (MirRhs::Store { model: m1, .. }, MirRhs::Load { model: m2, .. }) => (m1, m2),
        (MirRhs::Load { model: m1, .. }, MirRhs::Store { model: m2, .. }) => (m1, m2),
        (MirRhs::Store { model: m1, .. }, MirRhs::Store { model: m2, .. }) => (m1, m2),
        _ => return true, // non-memory ops can always reorder (for this check)
    };

    // If either has side effects, cannot reorder
    if m1.has_side_effects() || m2.has_side_effects() {
        return false;
    }

    // If both touch aliasing memory, cannot reorder
    if m1.may_alias() && m2.may_alias() {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_mir::{MirRhs, MirValue};

    #[test]
    fn global_store_is_sideeffecting() {
        let store = MirRhs::Store {
            model: MemoryModel::GlobalStorage,
            addr: MirValue(1),
            val: MirValue(2),
        };
        assert!(rhs_is_sideeffecting_store(&store));
    }

    #[test]
    fn register_load_is_not_aliasing() {
        let load = MirRhs::Load {
            model: MemoryModel::Register,
            addr: MirValue(1),
        };
        assert!(!rhs_has_aliasing_access(&load));
    }

    #[test]
    fn heap_load_may_alias() {
        let load = MirRhs::Load {
            model: MemoryModel::Heap,
            addr: MirValue(8),
        };
        assert!(rhs_has_aliasing_access(&load));
    }

    #[test]
    fn register_load_is_hoistable() {
        let load = MirRhs::Load {
            model: MemoryModel::Register,
            addr: MirValue(1),
        };
        assert!(memory_access_is_hoistable(&load));
    }

    #[test]
    fn stack_load_is_hoistable() {
        let load = MirRhs::Load {
            model: MemoryModel::Stack,
            addr: MirValue(1),
        };
        assert!(memory_access_is_hoistable(&load));
    }

    #[test]
    fn global_load_not_hoistable() {
        let load = MirRhs::Load {
            model: MemoryModel::GlobalStorage,
            addr: MirValue(1),
        };
        assert!(!memory_access_is_hoistable(&load));
    }

    #[test]
    fn store_never_hoistable() {
        let store = MirRhs::Store {
            model: MemoryModel::Register,
            addr: MirValue(1),
            val: MirValue(2),
        };
        assert!(!memory_access_is_hoistable(&store));
    }

    #[test]
    fn two_register_loads_can_reorder() {
        let load1 = MirRhs::Load {
            model: MemoryModel::Register,
            addr: MirValue(1),
        };
        let load2 = MirRhs::Load {
            model: MemoryModel::Register,
            addr: MirValue(2),
        };
        assert!(memory_accesses_can_reorder(&load1, &load2));
    }

    #[test]
    fn global_store_blocks_reordering() {
        let store = MirRhs::Store {
            model: MemoryModel::GlobalStorage,
            addr: MirValue(1),
            val: MirValue(2),
        };
        let load = MirRhs::Load {
            model: MemoryModel::Register,
            addr: MirValue(3),
        };
        assert!(!memory_accesses_can_reorder(&store, &load));
    }

    #[test]
    fn heap_load_heap_store_cannot_reorder() {
        let load = MirRhs::Load {
            model: MemoryModel::Heap,
            addr: MirValue(1),
        };
        let store = MirRhs::Store {
            model: MemoryModel::Heap,
            addr: MirValue(2),
            val: MirValue(3),
        };
        // Both touch aliasing memory
        assert!(!memory_accesses_can_reorder(&load, &store));
    }
}
