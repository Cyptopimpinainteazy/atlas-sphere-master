//! High-level Intermediate Representation (HIR) for the X3 language.
//!
//! HIR is the canonical typed representation used for:
//! - AI mutation (agents modify HIR, not AST or bytecode)
//! - Optimization passes
//! - MIR/bytecode lowering
//! - Cross-VM code generation
//!
//! Key properties:
//! - **Fully typed**: Every expression carries its resolved type
//! - **Fully desugared**: `for` → `while`, `match` expanded, etc.
//! - **Control-flow explicit**: No implicit fallthrough
//! - **Agent-safe**: Agent boundaries and lifecycle explicitly marked
//! - **Atomic-safe**: Atomic blocks use explicit begin/end markers
//! - **Deterministic**: Canonical ordering for reproducible compilation

use serde::{Deserialize, Serialize};
use std::fmt;

use x3_ast::{BinaryOp, UnaryOp};
use x3_common::{Literal, Span};
use x3_typeck::Type;

// ============================================================================
// Symbol Infrastructure
// ============================================================================

/// Compact identifier assigned during lowering for every declaration.
/// These IDs are stable within a module and used for efficient lookups.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SymbolId(pub usize);

impl SymbolId {
    pub const INVALID: SymbolId = SymbolId(usize::MAX);

    pub fn index(self) -> usize {
        self.0
    }

    pub fn is_valid(self) -> bool {
        self != Self::INVALID
    }
}

/// Label for control flow targets (loops, blocks).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LabelId(pub usize);

impl LabelId {
    pub fn index(self) -> usize {
        self.0
    }
}

/// Unique identifier for atomic transaction blocks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AtomicBlockId(pub usize);

/// Kinds of symbols tracked inside the HIR module.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolKind {
    /// Top-level function
    Function,
    /// Global constant or storage
    Global,
    /// Agent definition
    Agent,
    /// Local variable (stack-allocated)
    Local { mutable: bool },
    /// Function parameter
    Param { mutable: bool },
    /// Agent field
    AgentField { mutable: bool },
    /// Loop label for break/continue
    Label,
}

/// Complete symbol metadata for debugging, MIR lowering, and IDE support.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Symbol {
    pub id: SymbolId,
    pub name: String,
    pub kind: SymbolKind,
    /// Resolved type of this symbol.
    pub ty: Type,
    /// Span where symbol was declared.
    pub span: Span,
}

// ============================================================================
// Module Structure
// ============================================================================

/// Normalized module ready for MIR lowering.
/// Contains all top-level items with resolved types and symbols.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HirModule {
    /// Global constants and storage variables.
    pub globals: Vec<HirGlobal>,
    /// Top-level function definitions.
    pub functions: Vec<HirFunction>,
    /// Agent type definitions.
    pub agents: Vec<HirAgent>,
    /// All symbols in the module (flat lookup table).
    pub symbols: Vec<Symbol>,
    /// Source span of the entire module.
    pub span: Span,
}

/// Global constant or storage binding.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HirGlobal {
    pub symbol: SymbolId,
    pub ty: Type,
    pub initializer: HirExpr,
    pub span: Span,
}

/// Agent (smart contract / autonomous entity) definition.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HirAgent {
    pub symbol: SymbolId,
    /// Agent's storage fields.
    pub fields: Vec<HirAgentField>,
    /// Agent methods (including init).
    pub methods: Vec<HirFunction>,
    /// Optional init function symbol.
    pub init_fn: Option<SymbolId>,
    pub span: Span,
}

/// Field within an agent's storage.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HirAgentField {
    pub symbol: SymbolId,
    pub name: String,
    pub ty: Type,
    pub mutable: bool,
    pub span: Span,
}

/// A function with resolved parameters, body, and return type.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HirFunction {
    pub symbol: SymbolId,
    pub params: Vec<HirParam>,
    /// Function body as a sequence of statements.
    pub body: Vec<HirStmt>,
    /// Return type.
    pub return_ty: Type,
    /// Function attributes/annotations.
    pub attrs: FunctionAttrs,
    pub span: Span,
}

/// Function attributes for special behavior.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FunctionAttrs {
    /// This function is an agent's init function.
    pub is_init: bool,
    /// This function is a view (read-only, no state changes).
    pub is_view: bool,
    /// This function is payable (can receive native tokens).
    pub is_payable: bool,
    /// External visibility (callable from outside the module).
    pub is_external: bool,
    /// Target VM for this function (None = portable).
    pub target_vm: Option<TargetVm>,
}

/// Target VM for specialized code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetVm {
    Evm,
    Svm,
}

/// Function parameter with resolved type.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HirParam {
    pub symbol: SymbolId,
    pub name: String,
    pub ty: Type,
    pub mutable: bool,
    pub span: Span,
}

// ============================================================================
// Statements
// ============================================================================

/// Statement in HIR. All control flow is explicit.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum HirStmt {
    /// Local variable binding: `let x: T = expr;`
    Let {
        symbol: SymbolId,
        ty: Type,
        value: HirExpr,
        mutable: bool,
        span: Span,
    },

    /// Assignment: `target = value;`
    Assign {
        target: AssignTarget,
        value: HirExpr,
        span: Span,
    },

    /// Expression statement (for side effects).
    Expr(HirExpr),

    /// Return from function.
    Return { value: Option<HirExpr>, span: Span },

    /// Conditional branch.
    If {
        condition: HirExpr,
        then_block: Vec<HirStmt>,
        else_block: Vec<HirStmt>,
        span: Span,
    },

    /// While loop (also used for desugared `loop` and `for`).
    While {
        label: Option<LabelId>,
        condition: HirExpr,
        body: Vec<HirStmt>,
        span: Span,
    },

    /// Break out of a loop.
    Break { label: Option<LabelId>, span: Span },

    /// Continue to next iteration.
    Continue { label: Option<LabelId>, span: Span },

    // === X3-Specific Statements ===
    /// Begin an atomic transaction block.
    AtomicBegin { block_id: AtomicBlockId, span: Span },

    /// End an atomic transaction block.
    AtomicEnd {
        block_id: AtomicBlockId,
        /// If true, commit the transaction; if false, rollback.
        commit: bool,
        span: Span,
    },

    /// Emit an event.
    Emit {
        event_name: String,
        args: Vec<HirExpr>,
        span: Span,
    },

    /// Agent initialization (runs once when agent is deployed).
    AgentInit {
        agent_symbol: SymbolId,
        field_values: Vec<(SymbolId, HirExpr)>,
        span: Span,
    },
}

/// Target of an assignment.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AssignTarget {
    /// Simple variable: `x = ...`
    Variable(SymbolId),
    /// Field access: `obj.field = ...`
    Field {
        object: Box<HirExpr>,
        field: String,
        field_ty: Type,
    },
    /// Index access: `arr[i] = ...`
    Index {
        array: Box<HirExpr>,
        index: Box<HirExpr>,
        element_ty: Type,
    },
}

// ============================================================================
// Expressions
// ============================================================================

/// Typed expression in HIR. Every expression knows its type.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HirExpr {
    pub kind: HirExprKind,
    /// Resolved type of this expression.
    pub ty: Type,
    pub span: Span,
}

/// Expression kinds in HIR.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum HirExprKind {
    /// Literal value.
    Literal(Literal),

    /// Variable access.
    Var(SymbolId),

    /// Binary operation.
    Binary {
        op: BinaryOp,
        left: Box<HirExpr>,
        right: Box<HirExpr>,
    },

    /// Unary operation.
    Unary { op: UnaryOp, operand: Box<HirExpr> },

    /// Function call.
    Call {
        callee: SymbolId,
        args: Vec<HirExpr>,
    },

    /// Method call on an object.
    MethodCall {
        receiver: Box<HirExpr>,
        method: String,
        args: Vec<HirExpr>,
    },

    /// Field access.
    Field { object: Box<HirExpr>, field: String },

    /// Array/map index access.
    Index {
        array: Box<HirExpr>,
        index: Box<HirExpr>,
    },

    /// Array literal: `[1, 2, 3]`
    Array(Vec<HirExpr>),

    /// Tuple literal: `(a, b, c)`
    Tuple(Vec<HirExpr>),

    /// Block expression (returns last expression's value).
    Block {
        stmts: Vec<HirStmt>,
        /// Final expression (if any).
        expr: Option<Box<HirExpr>>,
    },

    /// If-else as an expression.
    IfExpr {
        condition: Box<HirExpr>,
        then_expr: Box<HirExpr>,
        else_expr: Box<HirExpr>,
    },

    /// Type cast (explicit conversion).
    Cast { expr: Box<HirExpr>, target_ty: Type },

    // === X3-Specific Expressions ===
    /// Context access: `context.sender`, `context.block_height`
    ContextAccess(ContextField),

    /// VM intrinsic call (EVM or SVM specific).
    VmIntrinsic {
        vm: TargetVm,
        intrinsic: VmIntrinsic,
        args: Vec<HirExpr>,
    },

    /// Agent self-reference within agent methods.
    SelfRef,
}

/// Built-in context fields accessible in X3.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextField {
    /// Transaction sender address.
    Sender,
    /// Current block height.
    BlockHeight,
    /// Current block timestamp.
    Timestamp,
    /// Transaction value (for payable functions).
    Value,
    /// Current gas remaining.
    GasRemaining,
    /// Chain ID (for cross-chain awareness).
    ChainId,
}

/// VM-specific intrinsic operations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VmIntrinsic {
    // EVM intrinsics
    EvmCall,
    EvmStaticCall,
    EvmDelegateCall,
    EvmCreate,
    EvmCreate2,
    EvmSload,
    EvmSstore,
    EvmLog,
    EvmBalance,
    EvmCodeSize,

    // SVM intrinsics
    SvmInvoke,
    SvmInvokeSigned,
    SvmCreateAccount,
    SvmTransfer,
    SvmGetAccountData,
    SvmSetAccountData,
    SvmGetRent,
    SvmGetClock,
}

// ============================================================================
// Impl Blocks
// ============================================================================

impl HirModule {
    /// Create an empty module.
    pub fn new(span: Span) -> Self {
        Self {
            globals: Vec::new(),
            functions: Vec::new(),
            agents: Vec::new(),
            symbols: Vec::new(),
            span,
        }
    }

    /// Lookup the human-readable name for a symbol ID.
    pub fn resolve_symbol(&self, symbol: SymbolId) -> Option<&Symbol> {
        self.symbols.iter().find(|entry| entry.id == symbol)
    }

    /// Get symbol name by ID.
    pub fn symbol_name(&self, symbol: SymbolId) -> Option<&str> {
        self.resolve_symbol(symbol).map(|s| s.name.as_str())
    }

    /// Get symbol type by ID.
    pub fn symbol_type(&self, symbol: SymbolId) -> Option<&Type> {
        self.resolve_symbol(symbol).map(|s| &s.ty)
    }

    /// Find a function by name.
    pub fn find_function(&self, name: &str) -> Option<&HirFunction> {
        self.functions
            .iter()
            .find(|f| self.symbol_name(f.symbol).map_or(false, |n| n == name))
    }

    /// Find an agent by name.
    pub fn find_agent(&self, name: &str) -> Option<&HirAgent> {
        self.agents
            .iter()
            .find(|a| self.symbol_name(a.symbol).map_or(false, |n| n == name))
    }
}

impl HirExpr {
    /// Create a new typed expression.
    pub fn new(kind: HirExprKind, ty: Type, span: Span) -> Self {
        Self { kind, ty, span }
    }

    /// Create a literal expression.
    pub fn literal(lit: Literal, ty: Type, span: Span) -> Self {
        Self::new(HirExprKind::Literal(lit), ty, span)
    }

    /// Create a variable access expression.
    pub fn var(symbol: SymbolId, ty: Type, span: Span) -> Self {
        Self::new(HirExprKind::Var(symbol), ty, span)
    }

    /// Create a binary expression.
    pub fn binary(op: BinaryOp, left: HirExpr, right: HirExpr, ty: Type, span: Span) -> Self {
        Self::new(
            HirExprKind::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            },
            ty,
            span,
        )
    }

    /// Create a function call expression.
    pub fn call(callee: SymbolId, args: Vec<HirExpr>, ty: Type, span: Span) -> Self {
        Self::new(HirExprKind::Call { callee, args }, ty, span)
    }

    /// Returns true if this expression has side effects.
    pub fn has_side_effects(&self) -> bool {
        match &self.kind {
            HirExprKind::Call { .. } | HirExprKind::MethodCall { .. } => true,
            HirExprKind::VmIntrinsic { .. } => true,
            HirExprKind::Block { stmts, .. } => !stmts.is_empty(),
            HirExprKind::Binary { left, right, .. } => {
                left.has_side_effects() || right.has_side_effects()
            }
            HirExprKind::Unary { operand, .. } => operand.has_side_effects(),
            HirExprKind::Index { array, index } => {
                array.has_side_effects() || index.has_side_effects()
            }
            HirExprKind::Field { object, .. } => object.has_side_effects(),
            _ => false,
        }
    }
}

impl HirStmt {
    /// Get the span of this statement.
    pub fn span(&self) -> Span {
        match self {
            HirStmt::Let { span, .. } => *span,
            HirStmt::Assign { span, .. } => *span,
            HirStmt::Expr(e) => e.span,
            HirStmt::Return { span, .. } => *span,
            HirStmt::If { span, .. } => *span,
            HirStmt::While { span, .. } => *span,
            HirStmt::Break { span, .. } => *span,
            HirStmt::Continue { span, .. } => *span,
            HirStmt::AtomicBegin { span, .. } => *span,
            HirStmt::AtomicEnd { span, .. } => *span,
            HirStmt::Emit { span, .. } => *span,
            HirStmt::AgentInit { span, .. } => *span,
        }
    }
}

impl fmt::Display for SymbolId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl fmt::Display for LabelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "L{}", self.0)
    }
}

impl fmt::Display for AtomicBlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "atomic_{}", self.0)
    }
}

impl fmt::Display for ContextField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContextField::Sender => write!(f, "sender"),
            ContextField::BlockHeight => write!(f, "block_height"),
            ContextField::Timestamp => write!(f, "timestamp"),
            ContextField::Value => write!(f, "value"),
            ContextField::GasRemaining => write!(f, "gas_remaining"),
            ContextField::ChainId => write!(f, "chain_id"),
        }
    }
}

impl fmt::Display for VmIntrinsic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VmIntrinsic::EvmCall => write!(f, "evm_call"),
            VmIntrinsic::EvmStaticCall => write!(f, "evm_static_call"),
            VmIntrinsic::EvmDelegateCall => write!(f, "evm_delegate_call"),
            VmIntrinsic::EvmCreate => write!(f, "evm_create"),
            VmIntrinsic::EvmCreate2 => write!(f, "evm_create2"),
            VmIntrinsic::EvmSload => write!(f, "evm_sload"),
            VmIntrinsic::EvmSstore => write!(f, "evm_sstore"),
            VmIntrinsic::EvmLog => write!(f, "evm_log"),
            VmIntrinsic::EvmBalance => write!(f, "evm_balance"),
            VmIntrinsic::EvmCodeSize => write!(f, "evm_code_size"),
            VmIntrinsic::SvmInvoke => write!(f, "svm_invoke"),
            VmIntrinsic::SvmInvokeSigned => write!(f, "svm_invoke_signed"),
            VmIntrinsic::SvmCreateAccount => write!(f, "svm_create_account"),
            VmIntrinsic::SvmTransfer => write!(f, "svm_transfer"),
            VmIntrinsic::SvmGetAccountData => write!(f, "svm_get_account_data"),
            VmIntrinsic::SvmSetAccountData => write!(f, "svm_set_account_data"),
            VmIntrinsic::SvmGetRent => write!(f, "svm_get_rent"),
            VmIntrinsic::SvmGetClock => write!(f, "svm_get_clock"),
        }
    }
}
