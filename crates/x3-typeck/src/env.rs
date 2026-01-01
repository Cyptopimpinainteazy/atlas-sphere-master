//! Type environment for tracking symbol types during type checking.

use std::collections::HashMap;

use x3_semantics::{ScopeId, SymbolId};

use crate::types::{AgentType, FunctionSignature, Type, TypeKind};

/// Binding of a symbol to its type.
#[derive(Clone, Debug)]
pub struct TypeBinding {
    /// The resolved type.
    pub ty: Type,
    /// Whether this binding is fully resolved (vs. still being inferred).
    pub resolved: bool,
}

impl TypeBinding {
    pub fn new(ty: Type) -> Self {
        Self { ty, resolved: true }
    }

    pub fn inferred(ty: Type) -> Self {
        Self {
            ty,
            resolved: false,
        }
    }
}

/// Type environment for a single scope.
#[derive(Clone, Debug, Default)]
struct ScopeTypeEnv {
    /// Types for symbols defined in this scope.
    bindings: HashMap<SymbolId, TypeBinding>,
}

/// The complete type environment, tracking types across all scopes.
#[derive(Clone, Debug)]
pub struct TypeEnv {
    /// Type bindings per scope.
    scopes: HashMap<ScopeId, ScopeTypeEnv>,

    /// Named type definitions (type aliases, agent types, etc.).
    named_types: HashMap<String, Type>,

    /// Function signatures by symbol ID.
    function_sigs: HashMap<SymbolId, FunctionSignature>,

    /// Agent type definitions by name.
    agent_types: HashMap<String, AgentType>,

    /// Next type variable ID for inference.
    next_type_var: u32,

    /// Substitutions from type variables to resolved types.
    substitutions: HashMap<u32, Type>,
}

impl Default for TypeEnv {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeEnv {
    /// Create a new empty type environment.
    pub fn new() -> Self {
        let mut env = Self {
            scopes: HashMap::new(),
            named_types: HashMap::new(),
            function_sigs: HashMap::new(),
            agent_types: HashMap::new(),
            next_type_var: 0,
            substitutions: HashMap::new(),
        };
        env.register_builtin_types();
        env
    }

    /// Register built-in type names.
    fn register_builtin_types(&mut self) {
        // Primitive type aliases
        self.named_types.insert("u8".to_string(), Type::u8());
        self.named_types.insert("u16".to_string(), Type::u16());
        self.named_types.insert("u32".to_string(), Type::u32());
        self.named_types.insert("u64".to_string(), Type::u64());
        self.named_types.insert("u128".to_string(), Type::u128());
        self.named_types.insert("u256".to_string(), Type::u256());
        self.named_types.insert("i8".to_string(), Type::i8());
        self.named_types.insert("i16".to_string(), Type::i16());
        self.named_types.insert("i32".to_string(), Type::i32());
        self.named_types.insert("i64".to_string(), Type::i64());
        self.named_types.insert("i128".to_string(), Type::i128());
        self.named_types.insert("bool".to_string(), Type::bool());
        self.named_types
            .insert("address".to_string(), Type::address());
        self.named_types
            .insert("pubkey".to_string(), Type::pubkey());
        self.named_types
            .insert("string".to_string(), Type::string());
        self.named_types.insert("bytes".to_string(), Type::bytes());

        // Special types
        self.named_types.insert("unit".to_string(), Type::unit());
        self.named_types.insert("never".to_string(), Type::never());
    }

    /// Ensure a scope exists in the environment.
    fn ensure_scope(&mut self, scope_id: ScopeId) {
        self.scopes.entry(scope_id).or_default();
    }

    /// Bind a symbol to a type in the given scope.
    pub fn bind(&mut self, scope_id: ScopeId, symbol_id: SymbolId, ty: Type) {
        self.ensure_scope(scope_id);
        if let Some(scope) = self.scopes.get_mut(&scope_id) {
            scope.bindings.insert(symbol_id, TypeBinding::new(ty));
        }
    }

    /// Bind a symbol with an inferred (potentially incomplete) type.
    pub fn bind_inferred(&mut self, scope_id: ScopeId, symbol_id: SymbolId, ty: Type) {
        self.ensure_scope(scope_id);
        if let Some(scope) = self.scopes.get_mut(&scope_id) {
            scope.bindings.insert(symbol_id, TypeBinding::inferred(ty));
        }
    }

    /// Look up a symbol's type.
    pub fn get(&self, symbol_id: SymbolId) -> Option<&Type> {
        // Search all scopes for the symbol
        for scope in self.scopes.values() {
            if let Some(binding) = scope.bindings.get(&symbol_id) {
                return Some(&binding.ty);
            }
        }
        None
    }

    /// Get a mutable reference to a symbol's type binding.
    pub fn get_mut(&mut self, symbol_id: SymbolId) -> Option<&mut TypeBinding> {
        for scope in self.scopes.values_mut() {
            if let Some(binding) = scope.bindings.get_mut(&symbol_id) {
                return Some(binding);
            }
        }
        None
    }

    /// Register a named type.
    pub fn register_type(&mut self, name: String, ty: Type) {
        self.named_types.insert(name, ty);
    }

    /// Look up a named type.
    pub fn lookup_type(&self, name: &str) -> Option<&Type> {
        self.named_types.get(name)
    }

    /// Register a function signature.
    pub fn register_function(&mut self, symbol_id: SymbolId, sig: FunctionSignature) {
        self.function_sigs.insert(symbol_id, sig);
    }

    /// Look up a function signature.
    pub fn get_function_sig(&self, symbol_id: SymbolId) -> Option<&FunctionSignature> {
        self.function_sigs.get(&symbol_id)
    }

    /// Register an agent type.
    pub fn register_agent(&mut self, name: String, agent: AgentType) {
        self.agent_types.insert(name, agent);
    }

    /// Look up an agent type.
    pub fn get_agent(&self, name: &str) -> Option<&AgentType> {
        self.agent_types.get(name)
    }

    // === Type Variable Management ===

    /// Create a fresh type variable.
    pub fn fresh_type_var(&mut self) -> Type {
        let id = self.next_type_var;
        self.next_type_var += 1;
        Type::type_var(id)
    }

    /// Record a substitution from a type variable to a concrete type.
    pub fn substitute(&mut self, var_id: u32, ty: Type) {
        self.substitutions.insert(var_id, ty);
    }

    /// Apply all substitutions to a type, resolving type variables.
    pub fn apply_substitutions(&self, ty: &Type) -> Type {
        match &ty.kind {
            TypeKind::TypeVar(id) => {
                if let Some(subst) = self.substitutions.get(id) {
                    // Recursively apply substitutions
                    self.apply_substitutions(subst)
                } else {
                    ty.clone()
                }
            }
            TypeKind::Function(sig) => {
                let params = sig
                    .params
                    .iter()
                    .map(|p| self.apply_substitutions(p))
                    .collect();
                let return_type = self.apply_substitutions(&sig.return_type);
                Type::new(TypeKind::Function(FunctionSignature {
                    params,
                    return_type: Box::new(return_type),
                    is_method: sig.is_method,
                }))
            }
            TypeKind::Array { element, size } => {
                Type::array(self.apply_substitutions(element), *size)
            }
            TypeKind::Vector(elem) => Type::vector(self.apply_substitutions(elem)),
            TypeKind::Option(inner) => Type::option(self.apply_substitutions(inner)),
            TypeKind::Result { ok, err } => Type::new(TypeKind::Result {
                ok: Box::new(self.apply_substitutions(ok)),
                err: Box::new(self.apply_substitutions(err)),
            }),
            TypeKind::Tuple(elems) => {
                Type::tuple(elems.iter().map(|e| self.apply_substitutions(e)).collect())
            }
            TypeKind::Atomic(inner) => Type::atomic(self.apply_substitutions(inner)),
            TypeKind::Context(inner) => Type::context(self.apply_substitutions(inner)),
            // Types that don't contain type variables
            _ => ty.clone(),
        }
    }

    /// Check if a type contains a specific type variable (occurs check).
    pub fn occurs_in(&self, var_id: u32, ty: &Type) -> bool {
        match &ty.kind {
            TypeKind::TypeVar(id) => {
                if *id == var_id {
                    return true;
                }
                // Check through substitutions
                if let Some(subst) = self.substitutions.get(id) {
                    return self.occurs_in(var_id, subst);
                }
                false
            }
            TypeKind::Function(sig) => {
                sig.params.iter().any(|p| self.occurs_in(var_id, p))
                    || self.occurs_in(var_id, &sig.return_type)
            }
            TypeKind::Array { element, .. } => self.occurs_in(var_id, element),
            TypeKind::Vector(elem) => self.occurs_in(var_id, elem),
            TypeKind::Option(inner) => self.occurs_in(var_id, inner),
            TypeKind::Result { ok, err } => {
                self.occurs_in(var_id, ok) || self.occurs_in(var_id, err)
            }
            TypeKind::Tuple(elems) => elems.iter().any(|e| self.occurs_in(var_id, e)),
            TypeKind::Atomic(inner) => self.occurs_in(var_id, inner),
            TypeKind::Context(inner) => self.occurs_in(var_id, inner),
            _ => false,
        }
    }

    /// Clear all substitutions (for testing or reset).
    pub fn clear_substitutions(&mut self) {
        self.substitutions.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_semantics::ScopeId;

    #[test]
    fn test_type_binding() {
        let mut env = TypeEnv::new();
        let scope = ScopeId(0);
        let symbol = SymbolId(0);

        env.bind(scope, symbol, Type::u64());

        let ty = env.get(symbol);
        assert!(ty.is_some());
        assert!(ty.unwrap().is_numeric());
    }

    #[test]
    fn test_builtin_types() {
        let env = TypeEnv::new();

        assert!(env.lookup_type("u64").is_some());
        assert!(env.lookup_type("bool").is_some());
        assert!(env.lookup_type("address").is_some());
        assert!(env.lookup_type("nonexistent").is_none());
    }

    #[test]
    fn test_type_variable_substitution() {
        let mut env = TypeEnv::new();

        let var = env.fresh_type_var();
        assert!(var.is_type_var());

        env.substitute(0, Type::u64());

        let resolved = env.apply_substitutions(&var);
        assert!(!resolved.is_type_var());
        assert!(resolved.is_numeric());
    }

    #[test]
    fn test_occurs_check() {
        let mut env = TypeEnv::new();

        let var = env.fresh_type_var();

        // Type variable doesn't occur in u64
        assert!(!env.occurs_in(0, &Type::u64()));

        // Type variable occurs in itself
        assert!(env.occurs_in(0, &var));

        // Type variable occurs in array containing it
        let arr = Type::array(var.clone(), 10);
        assert!(env.occurs_in(0, &arr));
    }
}
