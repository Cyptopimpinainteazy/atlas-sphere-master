//! Compilation options and optimization levels

pub use x3_opt::OptLevel;

/// Output format for compiler emit
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum EmitFormat {
    /// Emit bytecode (default)
    #[default]
    Bytecode,
    /// Emit MIR text representation
    Mir,
    /// Emit HIR text representation
    Hir,
    /// Emit AST
    Ast,
}

/// Compilation configuration
#[derive(Clone, Debug)]
pub struct CompilationOptions {
    /// Optimization level
    pub opt_level: OptLevel,
    /// Generate debug info
    pub debug: bool,
    /// Verbose output
    pub verbose: bool,

    // === Emit flags (for intermediate representation output) ===
    /// Emit HIR after lowering
    pub emit_hir: bool,
    /// Emit MIR before optimization
    pub emit_mir: bool,
    /// Emit optimized MIR
    pub emit_mir_opt: bool,
    /// Emit optimization statistics
    pub emit_stats: bool,
    /// Primary output format
    pub emit_format: EmitFormat,

    // === Contract analysis flags ===
    /// Run gas analysis on compiled code
    pub analyze_gas: bool,
    /// Verify contract safety (forbidden ops, determinism, etc.)
    pub verify_contract: bool,
}

impl Default for CompilationOptions {
    fn default() -> Self {
        Self {
            opt_level: OptLevel::Default,
            debug: false,
            verbose: false,
            emit_hir: false,
            emit_mir: false,
            emit_mir_opt: false,
            emit_stats: false,
            emit_format: EmitFormat::default(),
            analyze_gas: false,
            verify_contract: false,
        }
    }
}

impl CompilationOptions {
    /// No optimization
    pub fn no_opt() -> Self {
        Self {
            opt_level: OptLevel::None,
            ..Default::default()
        }
    }

    /// Basic optimization
    pub fn basic() -> Self {
        Self {
            opt_level: OptLevel::Basic,
            ..Default::default()
        }
    }

    /// Default optimization (O2)
    /// Includes: 13 YOLO passes + Loop-Pack v1 + PRE + Expression Hoisting
    pub fn opt2() -> Self {
        Self {
            opt_level: OptLevel::Default,
            ..Default::default()
        }
    }

    /// Aggressive optimization (O3)
    /// Includes: 13 YOLO passes + Loop-Pack v1 + PRE + Expression Hoisting (20 iterations)
    pub fn opt3() -> Self {
        Self {
            opt_level: OptLevel::Aggressive,
            ..Default::default()
        }
    }

    /// Contract mode: enables gas analysis and verification
    pub fn contract_mode() -> Self {
        Self {
            opt_level: OptLevel::Default,
            analyze_gas: true,
            verify_contract: true,
            ..Default::default()
        }
    }

    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    pub fn with_emit_hir(mut self, emit: bool) -> Self {
        self.emit_hir = emit;
        self
    }

    pub fn with_emit_mir(mut self, emit: bool) -> Self {
        self.emit_mir = emit;
        self
    }

    pub fn with_emit_mir_opt(mut self, emit: bool) -> Self {
        self.emit_mir_opt = emit;
        self
    }

    pub fn with_emit_stats(mut self, emit: bool) -> Self {
        self.emit_stats = emit;
        self
    }

    pub fn with_emit_format(mut self, format: EmitFormat) -> Self {
        self.emit_format = format;
        self
    }

    pub fn with_gas_analysis(mut self, analyze: bool) -> Self {
        self.analyze_gas = analyze;
        self
    }

    pub fn with_verification(mut self, verify: bool) -> Self {
        self.verify_contract = verify;
        self
    }
}
