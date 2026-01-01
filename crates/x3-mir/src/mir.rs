use x3_ast::{BinaryOp, UnaryOp};
use x3_common::Span;
pub use x3_hir::hir::SymbolId;

use crate::memory::MemoryModel;

/// Current MIR format version.
/// Increment MINOR for backward-compatible changes.
/// Increment MAJOR for breaking changes.
pub const MIR_VERSION_MAJOR: u16 = 1;
pub const MIR_VERSION_MINOR: u16 = 0;

/// MIR version tag for upgrade safety and compatibility checking.
/// Embedded in serialized MIR to detect version mismatches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MirVersion {
    /// Major version - breaking changes
    pub major: u16,
    /// Minor version - backward-compatible additions
    pub minor: u16,
}

impl MirVersion {
    /// Create a new MIR version
    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }

    /// Get current MIR version
    pub const fn current() -> Self {
        Self::new(MIR_VERSION_MAJOR, MIR_VERSION_MINOR)
    }

    /// Check if this version is compatible with another version.
    /// Compatible means same major version and self.minor >= other.minor
    pub fn is_compatible_with(&self, other: &MirVersion) -> bool {
        self.major == other.major && self.minor >= other.minor
    }

    /// Check if upgrade is required (other version is newer)
    pub fn needs_upgrade(&self, other: &MirVersion) -> bool {
        other.major > self.major || (other.major == self.major && other.minor > self.minor)
    }

    /// Encode version as u32 (major << 16 | minor)
    pub fn to_u32(&self) -> u32 {
        ((self.major as u32) << 16) | (self.minor as u32)
    }

    /// Decode version from u32
    pub fn from_u32(v: u32) -> Self {
        Self {
            major: (v >> 16) as u16,
            minor: (v & 0xFFFF) as u16,
        }
    }

    /// Format as semver-style string "major.minor"
    pub fn to_string(&self) -> alloc::string::String {
        alloc::format!("{}.{}", self.major, self.minor)
    }
}

impl Default for MirVersion {
    fn default() -> Self {
        Self::current()
    }
}

extern crate alloc;

/// SSA value produced inside the MIR module.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MirValue(pub usize);

/// Basic block identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MirBlockId(pub usize);

/// Lowered MIR module.
#[derive(Clone, Debug, PartialEq)]
pub struct MirModule {
    /// MIR format version for upgrade safety
    pub mir_version: MirVersion,
    /// Functions in this module
    pub functions: Vec<MirFunction>,
    /// Source span
    pub span: Span,
    /// Module metadata for diagnostics and debugging
    pub metadata: MirModuleMetadata,
}

impl MirModule {
    /// Create a new MIR module with current version
    pub fn new(functions: Vec<MirFunction>, span: Span) -> Self {
        Self {
            mir_version: MirVersion::current(),
            functions,
            span,
            metadata: MirModuleMetadata::default(),
        }
    }

    /// Create module with explicit version (for testing/migration)
    pub fn with_version(version: MirVersion, functions: Vec<MirFunction>, span: Span) -> Self {
        Self {
            mir_version: version,
            functions,
            span,
            metadata: MirModuleMetadata::default(),
        }
    }

    /// Check if this module is compatible with the current MIR version
    pub fn is_compatible(&self) -> bool {
        MirVersion::current().is_compatible_with(&self.mir_version)
    }

    /// Check if this module needs migration to current version
    pub fn needs_migration(&self) -> bool {
        self.mir_version.needs_upgrade(&MirVersion::current())
    }
}

/// Module metadata for diagnostics, debugging, and optimization hints
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MirModuleMetadata {
    /// Source file path (if available)
    pub source_path: Option<alloc::string::String>,
    /// Compiler version that produced this MIR
    pub compiler_version: Option<alloc::string::String>,
    /// Target VM (evm, svm, or both)
    pub target_vm: Option<TargetVm>,
    /// Optimization level applied
    pub optimization_level: u8,
    /// Whether debug info is preserved
    pub has_debug_info: bool,
}

/// Target virtual machine for code generation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TargetVm {
    /// Ethereum Virtual Machine
    Evm,
    /// Solana Virtual Machine
    Svm,
    /// Both VMs (dual-VM compilation)
    DualVm,
}

/// MIR-describing function body.
#[derive(Clone, Debug, PartialEq)]
pub struct MirFunction {
    pub symbol: SymbolId,
    pub params: Vec<MirValue>,
    pub entry: MirBlockId,
    pub blocks: Vec<MirBlock>,
    pub span: Span,
}

/// A basic block with statements and a terminator.
#[derive(Clone, Debug, PartialEq)]
pub struct MirBlock {
    pub id: MirBlockId,
    pub statements: Vec<MirStatement>,
    pub terminator: Option<MirTerminator>,
}

/// Side-effecting assignment (SSA binding) in a block.
#[derive(Clone, Debug, PartialEq)]
pub struct MirStatement {
    pub target: MirValue,
    pub rhs: MirRhs,
}

/// Right-hand sides for MIR assignments.
#[derive(Clone, Debug, PartialEq)]
pub enum MirRhs {
    Literal(x3_common::Literal),
    Unary(UnaryOp, MirValue),
    Binary(BinaryOp, MirValue, MirValue),
    Call {
        target: SymbolId,
        args: Vec<MirValue>,
    },
    /// Load from memory using the specified model.
    /// `addr` is the address/slot to load from.
    Load {
        model: MemoryModel,
        addr: MirValue,
    },
    /// Store to memory using the specified model.
    /// `addr` is the destination address/slot, `val` is the value to store.
    Store {
        model: MemoryModel,
        addr: MirValue,
        val: MirValue,
    },
}

/// Terminators that control the flow between blocks.
#[derive(Clone, Debug, PartialEq)]
pub enum MirTerminator {
    Return(Option<MirValue>),
    Goto(MirBlockId),
    Branch {
        cond: MirValue,
        then_block: MirBlockId,
        else_block: MirBlockId,
    },
}
