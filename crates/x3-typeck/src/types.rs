//! Type definitions for the X3 type system.
//!
//! This module defines the core type representation used throughout
//! the type checker, including primitives, compound types, and
//! blockchain-specific types for cross-VM interoperability.

use serde::{Deserialize, Serialize};
use std::fmt;
use x3_common::Span;

/// Unique identifier for a type in the type table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TypeId(pub usize);

impl TypeId {
    pub fn index(self) -> usize {
        self.0
    }
}

/// Primitive (scalar) types in X3.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PrimitiveType {
    // Unsigned integers
    U8,
    U16,
    U32,
    U64,
    U128,
    U256,

    // Signed integers
    I8,
    I16,
    I32,
    I64,
    I128,

    // Boolean
    Bool,

    // Blockchain addresses
    Address, // EVM 20-byte address
    Pubkey,  // SVM 32-byte public key

    // Fixed-size byte arrays
    Bytes32, // 32 bytes (common for hashes, keys)
}

impl PrimitiveType {
    /// Returns the size in bits for integer types.
    pub fn bit_width(&self) -> Option<u32> {
        match self {
            PrimitiveType::U8 | PrimitiveType::I8 => Some(8),
            PrimitiveType::U16 | PrimitiveType::I16 => Some(16),
            PrimitiveType::U32 | PrimitiveType::I32 => Some(32),
            PrimitiveType::U64 | PrimitiveType::I64 => Some(64),
            PrimitiveType::U128 | PrimitiveType::I128 => Some(128),
            PrimitiveType::U256 => Some(256),
            PrimitiveType::Bool => Some(1),
            PrimitiveType::Address => Some(160), // 20 bytes
            PrimitiveType::Pubkey => Some(256),  // 32 bytes
            PrimitiveType::Bytes32 => Some(256), // 32 bytes
        }
    }

    /// Returns true if this is a signed integer type.
    pub fn is_signed(&self) -> bool {
        matches!(
            self,
            PrimitiveType::I8
                | PrimitiveType::I16
                | PrimitiveType::I32
                | PrimitiveType::I64
                | PrimitiveType::I128
        )
    }

    /// Returns true if this is an integer type (signed or unsigned).
    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            PrimitiveType::U8
                | PrimitiveType::U16
                | PrimitiveType::U32
                | PrimitiveType::U64
                | PrimitiveType::U128
                | PrimitiveType::U256
                | PrimitiveType::I8
                | PrimitiveType::I16
                | PrimitiveType::I32
                | PrimitiveType::I64
                | PrimitiveType::I128
        )
    }

    /// Returns true if this is a numeric type (can be used in arithmetic).
    pub fn is_numeric(&self) -> bool {
        self.is_integer()
    }

    /// Returns true if this is an address/key type.
    pub fn is_address(&self) -> bool {
        matches!(self, PrimitiveType::Address | PrimitiveType::Pubkey)
    }

    /// Parse a primitive type from a string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "u8" => Some(PrimitiveType::U8),
            "u16" => Some(PrimitiveType::U16),
            "u32" => Some(PrimitiveType::U32),
            "u64" => Some(PrimitiveType::U64),
            "u128" => Some(PrimitiveType::U128),
            "u256" => Some(PrimitiveType::U256),
            "i8" => Some(PrimitiveType::I8),
            "i16" => Some(PrimitiveType::I16),
            "i32" => Some(PrimitiveType::I32),
            "i64" => Some(PrimitiveType::I64),
            "i128" => Some(PrimitiveType::I128),
            "bool" => Some(PrimitiveType::Bool),
            "address" => Some(PrimitiveType::Address),
            "pubkey" => Some(PrimitiveType::Pubkey),
            "bytes32" => Some(PrimitiveType::Bytes32),
            _ => None,
        }
    }
}

impl fmt::Display for PrimitiveType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrimitiveType::U8 => write!(f, "u8"),
            PrimitiveType::U16 => write!(f, "u16"),
            PrimitiveType::U32 => write!(f, "u32"),
            PrimitiveType::U64 => write!(f, "u64"),
            PrimitiveType::U128 => write!(f, "u128"),
            PrimitiveType::U256 => write!(f, "u256"),
            PrimitiveType::I8 => write!(f, "i8"),
            PrimitiveType::I16 => write!(f, "i16"),
            PrimitiveType::I32 => write!(f, "i32"),
            PrimitiveType::I64 => write!(f, "i64"),
            PrimitiveType::I128 => write!(f, "i128"),
            PrimitiveType::Bool => write!(f, "bool"),
            PrimitiveType::Address => write!(f, "address"),
            PrimitiveType::Pubkey => write!(f, "pubkey"),
            PrimitiveType::Bytes32 => write!(f, "bytes32"),
        }
    }
}

/// Cross-VM type category.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VmType {
    /// EVM-specific types
    Evm(EvmType),
    /// SVM (Solana) specific types
    Svm(SvmType),
    /// X3-native types
    X3(X3Type),
}

/// EVM-specific types.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EvmType {
    /// ERC20/ERC721 compatible asset
    Asset,
    /// Contract reference
    Contract,
    /// Storage slot
    StorageSlot,
}

/// SVM (Solana) specific types.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SvmType {
    /// Solana account
    Account,
    /// Program ID
    ProgramId,
    /// Instruction data
    InstructionData,
}

/// X3-native types.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum X3Type {
    /// Agent identifier
    AgentId,
    /// Atomic operation receipt
    AtomicReceipt,
    /// Cross-VM bridge handle
    BridgeHandle,
}

/// Function signature (parameter types + return type).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionSignature {
    /// Parameter types in order.
    pub params: Vec<Type>,
    /// Return type.
    pub return_type: Box<Type>,
    /// Whether this is a method (has implicit self).
    pub is_method: bool,
}

impl FunctionSignature {
    pub fn new(params: Vec<Type>, return_type: Type) -> Self {
        Self {
            params,
            return_type: Box::new(return_type),
            is_method: false,
        }
    }

    pub fn method(params: Vec<Type>, return_type: Type) -> Self {
        Self {
            params,
            return_type: Box::new(return_type),
            is_method: true,
        }
    }
}

/// Agent type definition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentType {
    /// Name of the agent type.
    pub name: String,
    /// Field names and types.
    pub fields: Vec<(String, Type)>,
    /// Method signatures.
    pub methods: Vec<(String, FunctionSignature)>,
}

/// The kind of a type - the actual type structure.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypeKind {
    /// Primitive scalar type.
    Primitive(PrimitiveType),

    /// Unit type (void, no value).
    Unit,

    /// Never type (function that doesn't return, e.g., panic).
    Never,

    /// Any type (for AI mutations on IR - NOT for user code).
    Any,

    /// Type variable (for inference).
    TypeVar(u32),

    /// Function type.
    Function(FunctionSignature),

    /// Agent type.
    Agent(AgentType),

    /// Fixed-size array: [T; N].
    Array { element: Box<Type>, size: usize },

    /// Dynamic array (vector).
    Vector(Box<Type>),

    /// Byte slice (dynamic size).
    Bytes,

    /// String type.
    String,

    /// Optional type: Option<T>.
    Option(Box<Type>),

    /// Result type: Result<T, E>.
    Result { ok: Box<Type>, err: Box<Type> },

    /// Tuple type: (T1, T2, ...).
    Tuple(Vec<Type>),

    /// Reference to a named type (before resolution).
    Named(String),

    /// Cross-VM type.
    Vm(VmType),

    /// Atomic wrapper type: atomic<T>.
    Atomic(Box<Type>),

    /// Context wrapper: context<T>.
    Context(Box<Type>),

    /// Error/unknown type (from failed inference).
    Error,
}

/// A complete type with its kind and optional source span.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Type {
    /// The kind/structure of this type.
    pub kind: TypeKind,
    /// Optional span where this type was declared/inferred.
    #[serde(skip)]
    pub span: Option<Span>,
}

impl Type {
    /// Create a new type with the given kind.
    pub fn new(kind: TypeKind) -> Self {
        Self { kind, span: None }
    }

    /// Create a type with a span.
    pub fn with_span(kind: TypeKind, span: Span) -> Self {
        Self {
            kind,
            span: Some(span),
        }
    }

    // === Convenience constructors ===

    pub fn unit() -> Self {
        Self::new(TypeKind::Unit)
    }

    pub fn never() -> Self {
        Self::new(TypeKind::Never)
    }

    pub fn any() -> Self {
        Self::new(TypeKind::Any)
    }

    pub fn error() -> Self {
        Self::new(TypeKind::Error)
    }

    pub fn bool() -> Self {
        Self::new(TypeKind::Primitive(PrimitiveType::Bool))
    }

    pub fn u8() -> Self {
        Self::new(TypeKind::Primitive(PrimitiveType::U8))
    }

    pub fn u16() -> Self {
        Self::new(TypeKind::Primitive(PrimitiveType::U16))
    }

    pub fn u32() -> Self {
        Self::new(TypeKind::Primitive(PrimitiveType::U32))
    }

    pub fn u64() -> Self {
        Self::new(TypeKind::Primitive(PrimitiveType::U64))
    }

    pub fn u128() -> Self {
        Self::new(TypeKind::Primitive(PrimitiveType::U128))
    }

    pub fn u256() -> Self {
        Self::new(TypeKind::Primitive(PrimitiveType::U256))
    }

    pub fn i8() -> Self {
        Self::new(TypeKind::Primitive(PrimitiveType::I8))
    }

    pub fn i16() -> Self {
        Self::new(TypeKind::Primitive(PrimitiveType::I16))
    }

    pub fn i32() -> Self {
        Self::new(TypeKind::Primitive(PrimitiveType::I32))
    }

    pub fn i64() -> Self {
        Self::new(TypeKind::Primitive(PrimitiveType::I64))
    }

    pub fn i128() -> Self {
        Self::new(TypeKind::Primitive(PrimitiveType::I128))
    }

    pub fn address() -> Self {
        Self::new(TypeKind::Primitive(PrimitiveType::Address))
    }

    pub fn pubkey() -> Self {
        Self::new(TypeKind::Primitive(PrimitiveType::Pubkey))
    }

    pub fn string() -> Self {
        Self::new(TypeKind::String)
    }

    pub fn bytes() -> Self {
        Self::new(TypeKind::Bytes)
    }

    pub fn type_var(id: u32) -> Self {
        Self::new(TypeKind::TypeVar(id))
    }

    pub fn function(params: Vec<Type>, return_type: Type) -> Self {
        Self::new(TypeKind::Function(FunctionSignature::new(
            params,
            return_type,
        )))
    }

    pub fn array(element: Type, size: usize) -> Self {
        Self::new(TypeKind::Array {
            element: Box::new(element),
            size,
        })
    }

    pub fn vector(element: Type) -> Self {
        Self::new(TypeKind::Vector(Box::new(element)))
    }

    pub fn option(inner: Type) -> Self {
        Self::new(TypeKind::Option(Box::new(inner)))
    }

    pub fn tuple(elements: Vec<Type>) -> Self {
        Self::new(TypeKind::Tuple(elements))
    }

    pub fn named(name: impl Into<String>) -> Self {
        Self::new(TypeKind::Named(name.into()))
    }

    pub fn atomic(inner: Type) -> Self {
        Self::new(TypeKind::Atomic(Box::new(inner)))
    }

    pub fn context(inner: Type) -> Self {
        Self::new(TypeKind::Context(Box::new(inner)))
    }

    // === Type queries ===

    /// Returns true if this is the unit type.
    pub fn is_unit(&self) -> bool {
        matches!(self.kind, TypeKind::Unit)
    }

    /// Returns true if this is the never type.
    pub fn is_never(&self) -> bool {
        matches!(self.kind, TypeKind::Never)
    }

    /// Returns true if this is a type variable (needs inference).
    pub fn is_type_var(&self) -> bool {
        matches!(self.kind, TypeKind::TypeVar(_))
    }

    /// Returns true if this is an error type.
    pub fn is_error(&self) -> bool {
        matches!(self.kind, TypeKind::Error)
    }

    /// Returns true if this is a primitive type.
    pub fn is_primitive(&self) -> bool {
        matches!(self.kind, TypeKind::Primitive(_))
    }

    /// Returns true if this is a numeric type.
    pub fn is_numeric(&self) -> bool {
        match &self.kind {
            TypeKind::Primitive(p) => p.is_numeric(),
            _ => false,
        }
    }

    /// Returns true if this is a boolean type.
    pub fn is_bool(&self) -> bool {
        matches!(self.kind, TypeKind::Primitive(PrimitiveType::Bool))
    }

    /// Returns true if this is a function type.
    pub fn is_function(&self) -> bool {
        matches!(self.kind, TypeKind::Function(_))
    }

    /// Returns the function signature if this is a function type.
    pub fn as_function(&self) -> Option<&FunctionSignature> {
        match &self.kind {
            TypeKind::Function(sig) => Some(sig),
            _ => None,
        }
    }

    /// Returns true if this type can be compared for equality.
    pub fn is_equatable(&self) -> bool {
        match &self.kind {
            TypeKind::Primitive(_) => true,
            TypeKind::Unit => true,
            TypeKind::String => true,
            TypeKind::Bytes => true,
            TypeKind::Array { element, .. } => element.is_equatable(),
            TypeKind::Tuple(elems) => elems.iter().all(|e| e.is_equatable()),
            TypeKind::Option(inner) => inner.is_equatable(),
            TypeKind::Vm(_) => true,
            _ => false,
        }
    }

    /// Returns true if this type can be ordered (< > <= >=).
    pub fn is_orderable(&self) -> bool {
        match &self.kind {
            TypeKind::Primitive(p) => p.is_numeric(),
            TypeKind::String => true,
            _ => false,
        }
    }

    /// Returns true if arithmetic operations are valid on this type.
    pub fn supports_arithmetic(&self) -> bool {
        self.is_numeric()
    }

    /// Returns true if bitwise operations are valid on this type.
    pub fn supports_bitwise(&self) -> bool {
        match &self.kind {
            TypeKind::Primitive(p) => p.is_integer(),
            _ => false,
        }
    }

    /// Returns true if logical operations are valid on this type.
    pub fn supports_logical(&self) -> bool {
        self.is_bool()
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            TypeKind::Primitive(p) => write!(f, "{}", p),
            TypeKind::Unit => write!(f, "()"),
            TypeKind::Never => write!(f, "!"),
            TypeKind::Any => write!(f, "any"),
            TypeKind::Error => write!(f, "<error>"),
            TypeKind::TypeVar(id) => write!(f, "?T{}", id),
            TypeKind::Function(sig) => {
                write!(f, "fn(")?;
                for (i, param) in sig.params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", param)?;
                }
                write!(f, ") -> {}", sig.return_type)
            }
            TypeKind::Agent(agent) => write!(f, "agent {}", agent.name),
            TypeKind::Array { element, size } => write!(f, "[{}; {}]", element, size),
            TypeKind::Vector(elem) => write!(f, "vec<{}>", elem),
            TypeKind::Bytes => write!(f, "bytes"),
            TypeKind::String => write!(f, "string"),
            TypeKind::Option(inner) => write!(f, "Option<{}>", inner),
            TypeKind::Result { ok, err } => write!(f, "Result<{}, {}>", ok, err),
            TypeKind::Tuple(elems) => {
                write!(f, "(")?;
                for (i, elem) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", elem)?;
                }
                write!(f, ")")
            }
            TypeKind::Named(name) => write!(f, "{}", name),
            TypeKind::Vm(vm) => match vm {
                VmType::Evm(evm) => match evm {
                    EvmType::Asset => write!(f, "evm::asset"),
                    EvmType::Contract => write!(f, "evm::contract"),
                    EvmType::StorageSlot => write!(f, "evm::storage_slot"),
                },
                VmType::Svm(svm) => match svm {
                    SvmType::Account => write!(f, "svm::account"),
                    SvmType::ProgramId => write!(f, "svm::program_id"),
                    SvmType::InstructionData => write!(f, "svm::instruction_data"),
                },
                VmType::X3(x3) => match x3 {
                    X3Type::AgentId => write!(f, "x3::agent_id"),
                    X3Type::AtomicReceipt => write!(f, "x3::atomic_receipt"),
                    X3Type::BridgeHandle => write!(f, "x3::bridge_handle"),
                },
            },
            TypeKind::Atomic(inner) => write!(f, "atomic<{}>", inner),
            TypeKind::Context(inner) => write!(f, "context<{}>", inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_properties() {
        assert!(PrimitiveType::U64.is_integer());
        assert!(PrimitiveType::U64.is_numeric());
        assert!(!PrimitiveType::U64.is_signed());

        assert!(PrimitiveType::I32.is_integer());
        assert!(PrimitiveType::I32.is_signed());

        assert!(!PrimitiveType::Bool.is_integer());
        assert!(!PrimitiveType::Bool.is_numeric());

        assert!(PrimitiveType::Address.is_address());
        assert!(PrimitiveType::Pubkey.is_address());
    }

    #[test]
    fn test_type_display() {
        assert_eq!(Type::u64().to_string(), "u64");
        assert_eq!(Type::bool().to_string(), "bool");
        assert_eq!(Type::unit().to_string(), "()");
        assert_eq!(Type::never().to_string(), "!");

        let fn_type = Type::function(vec![Type::u64(), Type::u64()], Type::u64());
        assert_eq!(fn_type.to_string(), "fn(u64, u64) -> u64");

        let arr_type = Type::array(Type::u8(), 32);
        assert_eq!(arr_type.to_string(), "[u8; 32]");
    }

    #[test]
    fn test_type_queries() {
        assert!(Type::u64().is_numeric());
        assert!(Type::i32().is_numeric());
        assert!(!Type::bool().is_numeric());

        assert!(Type::bool().is_bool());
        assert!(Type::bool().supports_logical());

        let fn_type = Type::function(vec![], Type::unit());
        assert!(fn_type.is_function());
        assert!(fn_type.as_function().is_some());
    }

    #[test]
    fn test_primitive_parsing() {
        assert_eq!(PrimitiveType::from_str("u64"), Some(PrimitiveType::U64));
        assert_eq!(PrimitiveType::from_str("bool"), Some(PrimitiveType::Bool));
        assert_eq!(
            PrimitiveType::from_str("address"),
            Some(PrimitiveType::Address)
        );
        assert_eq!(PrimitiveType::from_str("invalid"), None);
    }
}
