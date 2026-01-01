//! Symbol definitions and symbol table.

use serde::{Deserialize, Serialize};
use x3_common::Span;

use crate::scope::ScopeId;

/// Unique identifier for a symbol.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SymbolId(pub usize);

impl SymbolId {
    pub fn index(self) -> usize {
        self.0
    }
}

/// Kind of symbol in the symbol table.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolKind {
    /// A function definition.
    Function {
        /// Number of parameters.
        param_count: usize,
        /// Whether the function has a return type annotation.
        has_return_type: bool,
    },

    /// A local variable (let binding).
    Variable {
        /// Whether the binding is mutable.
        mutable: bool,
    },

    /// A constant binding.
    Constant,

    /// A function parameter.
    Parameter {
        /// Whether the parameter is mutable.
        mutable: bool,
        /// Parameter index (0-based).
        index: usize,
    },

    /// A global let binding.
    GlobalVariable {
        /// Whether the binding is mutable.
        mutable: bool,
    },

    /// An agent definition.
    Agent,

    /// A for-loop iteration variable.
    LoopVariable,
}

impl SymbolKind {
    /// Returns true if this symbol is mutable.
    pub fn is_mutable(&self) -> bool {
        match self {
            SymbolKind::Variable { mutable } => *mutable,
            SymbolKind::Parameter { mutable, .. } => *mutable,
            SymbolKind::GlobalVariable { mutable } => *mutable,
            SymbolKind::Constant => false,
            SymbolKind::Function { .. } => false,
            SymbolKind::Agent => false,
            SymbolKind::LoopVariable => false,
        }
    }

    /// Returns true if this symbol is callable (function).
    pub fn is_callable(&self) -> bool {
        matches!(self, SymbolKind::Function { .. })
    }
}

/// A symbol in the symbol table.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Symbol {
    /// Unique identifier for this symbol.
    pub id: SymbolId,

    /// The name of the symbol.
    pub name: String,

    /// The kind of symbol.
    pub kind: SymbolKind,

    /// The scope this symbol is defined in.
    pub scope: ScopeId,

    /// Span of the symbol's definition.
    pub def_span: Span,

    /// All spans where this symbol is referenced.
    #[serde(skip)]
    pub references: Vec<Span>,
}

impl Symbol {
    pub fn new(
        id: SymbolId,
        name: String,
        kind: SymbolKind,
        scope: ScopeId,
        def_span: Span,
    ) -> Self {
        Self {
            id,
            name,
            kind,
            scope,
            def_span,
            references: Vec::new(),
        }
    }

    /// Add a reference to this symbol.
    pub fn add_reference(&mut self, span: Span) {
        self.references.push(span);
    }
}

/// Symbol table that stores all symbols across all scopes.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SymbolTable {
    /// All symbols indexed by SymbolId.
    symbols: Vec<Symbol>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            symbols: Vec::new(),
        }
    }

    /// Insert a new symbol and return its ID.
    pub fn insert(&mut self, symbol: Symbol) -> SymbolId {
        let id = SymbolId(self.symbols.len());
        debug_assert_eq!(symbol.id, id);
        self.symbols.push(symbol);
        id
    }

    /// Create a new symbol with the next available ID.
    pub fn create(
        &mut self,
        name: String,
        kind: SymbolKind,
        scope: ScopeId,
        def_span: Span,
    ) -> SymbolId {
        let id = SymbolId(self.symbols.len());
        let symbol = Symbol::new(id, name, kind, scope, def_span);
        self.symbols.push(symbol);
        id
    }

    /// Get a symbol by ID.
    pub fn get(&self, id: SymbolId) -> Option<&Symbol> {
        self.symbols.get(id.0)
    }

    /// Get a mutable reference to a symbol by ID.
    pub fn get_mut(&mut self, id: SymbolId) -> Option<&mut Symbol> {
        self.symbols.get_mut(id.0)
    }

    /// Iterate over all symbols.
    pub fn iter(&self) -> impl Iterator<Item = &Symbol> {
        self.symbols.iter()
    }

    /// Get the number of symbols.
    pub fn len(&self) -> usize {
        self.symbols.len()
    }

    /// Check if the table is empty.
    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }

    /// Add a reference to a symbol.
    pub fn add_reference(&mut self, id: SymbolId, span: Span) {
        if let Some(symbol) = self.get_mut(id) {
            symbol.add_reference(span);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_table_basics() {
        let mut table = SymbolTable::new();

        let id = table.create(
            "x".to_string(),
            SymbolKind::Variable { mutable: false },
            ScopeId(0),
            Span::new(0, 1),
        );

        assert_eq!(id, SymbolId(0));
        assert_eq!(table.len(), 1);

        let symbol = table.get(id).unwrap();
        assert_eq!(symbol.name, "x");
        assert!(!symbol.kind.is_mutable());
    }

    #[test]
    fn test_symbol_mutability() {
        let mutable_var = SymbolKind::Variable { mutable: true };
        let immutable_var = SymbolKind::Variable { mutable: false };
        let constant = SymbolKind::Constant;
        let function = SymbolKind::Function {
            param_count: 2,
            has_return_type: true,
        };

        assert!(mutable_var.is_mutable());
        assert!(!immutable_var.is_mutable());
        assert!(!constant.is_mutable());
        assert!(!function.is_mutable());
        assert!(function.is_callable());
    }
}
