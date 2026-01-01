//! Safety rules configuration
//!
//! Loads and manages contract safety rules from YAML configuration.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

use crate::error::{VerifierError, VerifierResult};

/// Classification of opcodes by safety level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpcodeClass {
    /// Always safe - arithmetic, basic memory, control flow
    Safe,
    /// Requires careful use - storage, calls, crypto
    Restricted,
    /// Never allowed in contracts
    Forbidden,
}

impl Default for OpcodeClass {
    fn default() -> Self {
        OpcodeClass::Forbidden // Default to forbidden for unknown ops
    }
}

/// Gas cost definition for an opcode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasCost {
    /// Base cost of the operation
    pub base: u64,
    /// Per-byte cost for variable-size operations
    #[serde(default)]
    pub per_byte: u64,
    /// Whether cost scales with input size
    #[serde(default)]
    pub dynamic: bool,
}

impl Default for GasCost {
    fn default() -> Self {
        GasCost {
            base: 1,
            per_byte: 0,
            dynamic: false,
        }
    }
}

/// Limits for various contract resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Limits {
    /// Maximum gas per function
    #[serde(default = "default_max_function_gas")]
    pub max_function_gas: u64,

    /// Maximum gas per contract (all functions)
    #[serde(default = "default_max_contract_gas")]
    pub max_contract_gas: u64,

    /// Maximum instructions per function
    #[serde(default = "default_max_instructions")]
    pub max_instructions_per_function: usize,

    /// Maximum call depth
    #[serde(default = "default_max_call_depth")]
    pub max_call_depth: usize,

    /// Maximum storage slots per contract
    #[serde(default = "default_max_storage_slots")]
    pub max_storage_slots: usize,

    /// Maximum bytecode size in bytes
    #[serde(default = "default_max_bytecode_size")]
    pub max_bytecode_size: usize,
}

fn default_max_function_gas() -> u64 {
    10_000_000
}
fn default_max_contract_gas() -> u64 {
    100_000_000
}
fn default_max_instructions() -> usize {
    100_000
}
fn default_max_call_depth() -> usize {
    64
}
fn default_max_storage_slots() -> usize {
    1000
}
fn default_max_bytecode_size() -> usize {
    1_048_576
} // 1MB

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_function_gas: default_max_function_gas(),
            max_contract_gas: default_max_contract_gas(),
            max_instructions_per_function: default_max_instructions(),
            max_call_depth: default_max_call_depth(),
            max_storage_slots: default_max_storage_slots(),
            max_bytecode_size: default_max_bytecode_size(),
        }
    }
}

/// Atomic execution rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomicRules {
    /// Maximum gas within atomic block
    #[serde(default = "default_atomic_max_gas")]
    pub max_gas: u64,

    /// Maximum duration in milliseconds
    #[serde(default = "default_atomic_max_duration")]
    pub max_duration_ms: u64,

    /// Allowed operations within atomic blocks
    #[serde(default)]
    pub allowed_ops: Vec<String>,

    /// Forbidden operations within atomic blocks
    #[serde(default)]
    pub forbidden_ops: Vec<String>,
}

fn default_atomic_max_gas() -> u64 {
    5_000_000
}
fn default_atomic_max_duration() -> u64 {
    500
}

impl Default for AtomicRules {
    fn default() -> Self {
        AtomicRules {
            max_gas: default_atomic_max_gas(),
            max_duration_ms: default_atomic_max_duration(),
            allowed_ops: vec![],
            forbidden_ops: vec![],
        }
    }
}

/// Determinism requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeterminismRules {
    /// Require deterministic execution
    #[serde(default = "default_true")]
    pub required: bool,

    /// Forbidden sources of non-determinism
    #[serde(default)]
    pub forbidden_sources: Vec<String>,
}

fn default_true() -> bool {
    true
}

impl Default for DeterminismRules {
    fn default() -> Self {
        DeterminismRules {
            required: true,
            forbidden_sources: vec![
                "floating_point".to_string(),
                "system_time".to_string(),
                "random".to_string(),
                "network_io".to_string(),
                "file_io".to_string(),
            ],
        }
    }
}

/// Complete safety rules configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyRules {
    /// Version of the rules format
    #[serde(default = "default_version")]
    pub version: String,

    /// Opcode classifications
    #[serde(default)]
    pub opcodes: BTreeMap<String, OpcodeClass>,

    /// Gas costs per opcode
    #[serde(default)]
    pub gas_costs: BTreeMap<String, GasCost>,

    /// Resource limits
    #[serde(default)]
    pub limits: Limits,

    /// Atomic execution rules
    #[serde(default)]
    pub atomic: AtomicRules,

    /// Determinism requirements
    #[serde(default)]
    pub determinism: DeterminismRules,

    /// Custom forbidden operations (explicit list)
    #[serde(default)]
    pub forbidden_ops: Vec<String>,

    /// Operations restricted in certain contexts
    #[serde(default)]
    pub restricted_ops: Vec<String>,
}

fn default_version() -> String {
    "1.0".to_string()
}

impl Default for SafetyRules {
    fn default() -> Self {
        let mut opcodes = BTreeMap::new();
        let mut gas_costs = BTreeMap::new();

        // Safe operations
        for op in &[
            "add", "sub", "mul", "div", "rem", "and", "or", "xor", "shl", "shr", "eq", "ne", "lt",
            "gt", "le", "ge", "load", "store", "const", "copy", "jump", "branch", "call", "return",
            "nop",
        ] {
            opcodes.insert(op.to_string(), OpcodeClass::Safe);
        }

        // Restricted operations
        for op in &[
            "sload",
            "sstore",
            "log",
            "call_external",
            "delegate_call",
            "create",
            "keccak256",
            "sha256",
            "blake2",
            "ecrecover",
        ] {
            opcodes.insert(op.to_string(), OpcodeClass::Restricted);
        }

        // Forbidden operations
        for op in &[
            "selfdestruct",
            "delegatecall_unchecked",
            "arbitrary_jump",
            "timestamp",
            "block_hash_old",
            "coinbase",
        ] {
            opcodes.insert(op.to_string(), OpcodeClass::Forbidden);
        }

        // Default gas costs
        gas_costs.insert(
            "add".to_string(),
            GasCost {
                base: 3,
                per_byte: 0,
                dynamic: false,
            },
        );
        gas_costs.insert(
            "sub".to_string(),
            GasCost {
                base: 3,
                per_byte: 0,
                dynamic: false,
            },
        );
        gas_costs.insert(
            "mul".to_string(),
            GasCost {
                base: 5,
                per_byte: 0,
                dynamic: false,
            },
        );
        gas_costs.insert(
            "div".to_string(),
            GasCost {
                base: 5,
                per_byte: 0,
                dynamic: false,
            },
        );
        gas_costs.insert(
            "sload".to_string(),
            GasCost {
                base: 200,
                per_byte: 0,
                dynamic: false,
            },
        );
        gas_costs.insert(
            "sstore".to_string(),
            GasCost {
                base: 5000,
                per_byte: 0,
                dynamic: false,
            },
        );
        gas_costs.insert(
            "keccak256".to_string(),
            GasCost {
                base: 30,
                per_byte: 6,
                dynamic: true,
            },
        );
        gas_costs.insert(
            "call_external".to_string(),
            GasCost {
                base: 700,
                per_byte: 0,
                dynamic: false,
            },
        );

        SafetyRules {
            version: default_version(),
            opcodes,
            gas_costs,
            limits: Limits::default(),
            atomic: AtomicRules::default(),
            determinism: DeterminismRules::default(),
            forbidden_ops: vec![
                "selfdestruct".to_string(),
                "delegatecall_unchecked".to_string(),
            ],
            restricted_ops: vec![
                "sstore".to_string(),
                "call_external".to_string(),
                "create".to_string(),
            ],
        }
    }
}

impl SafetyRules {
    /// Load rules from a YAML file
    pub fn load<P: AsRef<Path>>(path: P) -> VerifierResult<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| VerifierError::RulesLoad(e.to_string()))?;
        Self::from_yaml(&content)
    }

    /// Parse rules from YAML string
    pub fn from_yaml(yaml: &str) -> VerifierResult<Self> {
        serde_yaml::from_str(yaml).map_err(|e| VerifierError::RulesParse(e.to_string()))
    }

    /// Parse rules from JSON string
    pub fn from_json(json: &str) -> VerifierResult<Self> {
        serde_json::from_str(json).map_err(|e| VerifierError::RulesParse(e.to_string()))
    }

    /// Get the classification of an opcode
    pub fn classify_opcode(&self, opcode: &str) -> OpcodeClass {
        self.opcodes
            .get(opcode)
            .copied()
            .unwrap_or(OpcodeClass::Forbidden)
    }

    /// Get the gas cost of an opcode
    pub fn gas_cost(&self, opcode: &str) -> GasCost {
        self.gas_costs.get(opcode).cloned().unwrap_or_default()
    }

    /// Check if an opcode is forbidden
    pub fn is_forbidden(&self, opcode: &str) -> bool {
        self.classify_opcode(opcode) == OpcodeClass::Forbidden
            || self.forbidden_ops.contains(&opcode.to_string())
    }

    /// Check if an opcode is restricted
    pub fn is_restricted(&self, opcode: &str) -> bool {
        self.classify_opcode(opcode) == OpcodeClass::Restricted
            || self.restricted_ops.contains(&opcode.to_string())
    }

    /// Check if an opcode is safe in all contexts
    pub fn is_safe(&self, opcode: &str) -> bool {
        self.classify_opcode(opcode) == OpcodeClass::Safe
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_rules() {
        let rules = SafetyRules::default();

        assert!(rules.is_safe("add"));
        assert!(rules.is_safe("mul"));
        assert!(rules.is_restricted("sstore"));
        assert!(rules.is_forbidden("selfdestruct"));
    }

    #[test]
    fn test_gas_costs() {
        let rules = SafetyRules::default();

        assert_eq!(rules.gas_cost("add").base, 3);
        assert_eq!(rules.gas_cost("sstore").base, 5000);
        assert!(rules.gas_cost("keccak256").dynamic);
    }

    #[test]
    fn test_limits() {
        let rules = SafetyRules::default();

        assert_eq!(rules.limits.max_function_gas, 10_000_000);
        assert_eq!(rules.limits.max_call_depth, 64);
    }
}
