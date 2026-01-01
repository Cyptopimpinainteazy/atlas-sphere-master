//! Scope management for semantic analysis.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use x3_common::Span;

use crate::symbol::SymbolId;

/// Unique identifier for a scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScopeId(pub usize);

impl ScopeId {
    pub fn index(self) -> usize {
        self.0
    }

    /// The root/global scope ID.
    pub const ROOT: ScopeId = ScopeId(0);
}

/// Kind of scope - determines what constructs are valid within.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScopeKind {
    /// Global/module scope - top level declarations.
    Global,

    /// Function body scope.
    Function,

    /// Block scope (if, while, loop, for bodies).
    Block,

    /// Loop scope (while, loop, for) - allows break/continue.
    Loop,

    /// Agent scope - contains agent-local items.
    Agent,

    /// Atomic block scope.
    Atomic,
}

impl ScopeKind {
    /// Returns true if break/continue are valid in this scope kind.
    pub fn allows_break_continue(&self) -> bool {
        matches!(self, ScopeKind::Loop)
    }

    /// Returns true if return is valid in this scope kind.
    pub fn allows_return(&self) -> bool {
        matches!(self, ScopeKind::Function)
    }
}

/// A scope in the scope tree.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Scope {
    /// Unique identifier for this scope.
    pub id: ScopeId,

    /// The kind of scope.
    pub kind: ScopeKind,

    /// Parent scope (None for global scope).
    pub parent: Option<ScopeId>,

    /// Span of the scope (e.g., function body, block).
    pub span: Span,

    /// Symbols defined directly in this scope (name -> SymbolId).
    #[serde(skip)]
    pub symbols: HashMap<String, SymbolId>,

    /// Child scopes.
    #[serde(skip)]
    pub children: Vec<ScopeId>,

    /// Depth in the scope tree (0 for global).
    pub depth: usize,
}

impl Scope {
    pub fn new(id: ScopeId, kind: ScopeKind, parent: Option<ScopeId>, span: Span) -> Self {
        let depth = if parent.is_some() { 1 } else { 0 }; // Will be set properly by ScopeTree
        Self {
            id,
            kind,
            parent,
            span,
            symbols: HashMap::new(),
            children: Vec::new(),
            depth,
        }
    }

    /// Define a symbol in this scope.
    pub fn define(&mut self, name: String, symbol_id: SymbolId) -> Option<SymbolId> {
        self.symbols.insert(name, symbol_id)
    }

    /// Look up a symbol in this scope only (not parent scopes).
    pub fn lookup_local(&self, name: &str) -> Option<SymbolId> {
        self.symbols.get(name).copied()
    }
}

/// Tree of scopes for the entire module.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ScopeTree {
    /// All scopes indexed by ScopeId.
    scopes: Vec<Scope>,
}

impl ScopeTree {
    pub fn new() -> Self {
        let mut tree = Self { scopes: Vec::new() };
        // Create the global scope
        tree.create_scope(ScopeKind::Global, None, Span::default());
        tree
    }

    /// Create a new scope and return its ID.
    pub fn create_scope(
        &mut self,
        kind: ScopeKind,
        parent: Option<ScopeId>,
        span: Span,
    ) -> ScopeId {
        let id = ScopeId(self.scopes.len());
        let depth = parent
            .and_then(|p| self.get(p))
            .map(|p| p.depth + 1)
            .unwrap_or(0);

        let mut scope = Scope::new(id, kind, parent, span);
        scope.depth = depth;

        self.scopes.push(scope);

        // Add as child to parent
        if let Some(parent_id) = parent {
            if let Some(parent_scope) = self.get_mut(parent_id) {
                parent_scope.children.push(id);
            }
        }

        id
    }

    /// Get a scope by ID.
    pub fn get(&self, id: ScopeId) -> Option<&Scope> {
        self.scopes.get(id.0)
    }

    /// Get a mutable reference to a scope.
    pub fn get_mut(&mut self, id: ScopeId) -> Option<&mut Scope> {
        self.scopes.get_mut(id.0)
    }

    /// Define a symbol in a scope.
    pub fn define(
        &mut self,
        scope_id: ScopeId,
        name: String,
        symbol_id: SymbolId,
    ) -> Option<SymbolId> {
        self.get_mut(scope_id)
            .and_then(|scope| scope.define(name, symbol_id))
    }

    /// Look up a symbol starting from the given scope, walking up the parent chain.
    pub fn lookup(&self, start_scope: ScopeId, name: &str) -> Option<SymbolId> {
        let mut current = Some(start_scope);
        while let Some(scope_id) = current {
            if let Some(scope) = self.get(scope_id) {
                if let Some(symbol_id) = scope.lookup_local(name) {
                    return Some(symbol_id);
                }
                current = scope.parent;
            } else {
                break;
            }
        }
        None
    }

    /// Check if we're inside a loop scope (walking up the parent chain).
    pub fn is_in_loop(&self, scope_id: ScopeId) -> bool {
        let mut current = Some(scope_id);
        while let Some(id) = current {
            if let Some(scope) = self.get(id) {
                if scope.kind == ScopeKind::Loop {
                    return true;
                }
                current = scope.parent;
            } else {
                break;
            }
        }
        false
    }

    /// Check if we're inside a function scope (walking up the parent chain).
    pub fn is_in_function(&self, scope_id: ScopeId) -> bool {
        let mut current = Some(scope_id);
        while let Some(id) = current {
            if let Some(scope) = self.get(id) {
                if scope.kind == ScopeKind::Function {
                    return true;
                }
                current = scope.parent;
            } else {
                break;
            }
        }
        false
    }

    /// Check if we're inside an atomic scope (walking up the parent chain).
    pub fn is_in_atomic(&self, scope_id: ScopeId) -> bool {
        let mut current = Some(scope_id);
        while let Some(id) = current {
            if let Some(scope) = self.get(id) {
                if scope.kind == ScopeKind::Atomic {
                    return true;
                }
                current = scope.parent;
            } else {
                break;
            }
        }
        false
    }

    /// Get the global scope.
    pub fn global_scope(&self) -> &Scope {
        self.get(ScopeId::ROOT).expect("global scope must exist")
    }

    /// Get the number of scopes.
    pub fn len(&self) -> usize {
        self.scopes.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.scopes.is_empty()
    }

    /// Iterate over all scopes.
    pub fn iter(&self) -> impl Iterator<Item = &Scope> {
        self.scopes.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_tree_creation() {
        let tree = ScopeTree::new();
        assert_eq!(tree.len(), 1);
        assert_eq!(tree.global_scope().kind, ScopeKind::Global);
        assert_eq!(tree.global_scope().depth, 0);
    }

    #[test]
    fn test_nested_scopes() {
        let mut tree = ScopeTree::new();

        let func_scope =
            tree.create_scope(ScopeKind::Function, Some(ScopeId::ROOT), Span::new(0, 100));
        let loop_scope = tree.create_scope(ScopeKind::Loop, Some(func_scope), Span::new(10, 90));

        assert!(tree.is_in_function(loop_scope));
        assert!(tree.is_in_loop(loop_scope));
        assert!(!tree.is_in_loop(func_scope));

        assert_eq!(tree.get(func_scope).unwrap().depth, 1);
        assert_eq!(tree.get(loop_scope).unwrap().depth, 2);
    }

    #[test]
    fn test_symbol_lookup() {
        let mut tree = ScopeTree::new();
        let func_scope =
            tree.create_scope(ScopeKind::Function, Some(ScopeId::ROOT), Span::new(0, 100));
        let block_scope = tree.create_scope(ScopeKind::Block, Some(func_scope), Span::new(10, 90));

        // Define 'x' in global scope
        tree.define(ScopeId::ROOT, "x".to_string(), SymbolId(0));
        // Define 'y' in function scope
        tree.define(func_scope, "y".to_string(), SymbolId(1));

        // Can find 'x' from block scope (walks up)
        assert_eq!(tree.lookup(block_scope, "x"), Some(SymbolId(0)));
        // Can find 'y' from block scope
        assert_eq!(tree.lookup(block_scope, "y"), Some(SymbolId(1)));
        // Cannot find 'y' from global scope
        assert_eq!(tree.lookup(ScopeId::ROOT, "y"), None);
    }
}
