//! X3 VM Executor

use crate::config::VmConfig;
use thiserror::Error;

/// Execution result
#[derive(Clone, Debug)]
pub struct ExecutionResult {
    /// Success flag
    pub success: bool,
    /// Gas used
    pub gas_used: u64,
    /// Return data
    pub return_data: Vec<u8>,
    /// Logs emitted
    pub logs: Vec<ExecutionLog>,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// Execution log
#[derive(Clone, Debug)]
pub struct ExecutionLog {
    pub topics: Vec<[u8; 32]>,
    pub data: Vec<u8>,
}

/// Execution error
#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("Out of gas")]
    OutOfGas,
    #[error("Invalid bytecode: {0}")]
    InvalidBytecode(String),
    #[error("Runtime error: {0}")]
    RuntimeError(String),
    #[error("Memory limit exceeded")]
    MemoryLimitExceeded,
    #[error("Stack overflow")]
    StackOverflow,
}

/// X3 VM Executor
pub struct X3Executor {
    config: VmConfig,
}

impl X3Executor {
    /// Create a new executor
    pub fn new(config: VmConfig) -> Self {
        Self { config }
    }

    /// Execute bytecode with input and gas limit
    pub fn execute(
        &self,
        bytecode: &[u8],
        input: &[u8],
        gas_limit: u64,
    ) -> Result<ExecutionResult, ExecutionError> {
        tracing::debug!(
            "Executing {} bytes of bytecode with {} gas limit",
            bytecode.len(),
            gas_limit
        );

        // Basic bytecode validation
        if bytecode.len() < 4 {
            return Err(ExecutionError::InvalidBytecode("Bytecode too short".into()));
        }

        // Check magic bytes (X3VC = 0x58335643)
        if bytecode[0..4] != [0x58, 0x33, 0x56, 0x43] {
            return Err(ExecutionError::InvalidBytecode(
                "Invalid magic bytes".into(),
            ));
        }

        // Execute bytecode
        let mut gas_remaining = gas_limit.min(self.config.max_gas);
        let mut output = Vec::new();
        let mut logs = Vec::new();

        match self.execute_inner(bytecode, input, &mut gas_remaining, &mut output, &mut logs) {
            Ok(()) => Ok(ExecutionResult {
                success: true,
                gas_used: gas_limit - gas_remaining,
                return_data: output,
                logs,
                error: None,
            }),
            Err(e) => Ok(ExecutionResult {
                success: false,
                gas_used: gas_limit - gas_remaining,
                return_data: Vec::new(),
                logs,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Inner execution loop
    fn execute_inner(
        &self,
        bytecode: &[u8],
        input: &[u8],
        gas_remaining: &mut u64,
        output: &mut Vec<u8>,
        logs: &mut Vec<ExecutionLog>,
    ) -> Result<(), ExecutionError> {
        // Simplified execution - in production uses full x3-vm
        let mut pc = 12; // Skip header (magic + version + flags + padding)
        let mut stack: Vec<i64> = Vec::new();
        let mut registers: [i64; 32] = [0; 32];
        let mut memory = vec![0u8; 1024]; // 1KB initial memory

        // Load input into memory at offset 0
        if !input.is_empty() {
            let len = input.len().min(memory.len());
            memory[..len].copy_from_slice(&input[..len]);
        }

        while pc < bytecode.len() {
            // Gas check
            if *gas_remaining == 0 {
                return Err(ExecutionError::OutOfGas);
            }

            let opcode = bytecode[pc];
            *gas_remaining = gas_remaining.saturating_sub(1);

            match opcode {
                // NOP
                0xA0 => pc += 1,

                // HALT
                0xA1 => break,

                // PUSH_CONST_I16
                0x01 => {
                    if pc + 2 >= bytecode.len() {
                        return Err(ExecutionError::InvalidBytecode("Truncated PUSH".into()));
                    }
                    let val = i16::from_le_bytes([bytecode[pc + 1], bytecode[pc + 2]]) as i64;
                    stack.push(val);
                    pc += 3;
                }

                // POP to register
                0x03 => {
                    if pc + 1 >= bytecode.len() {
                        return Err(ExecutionError::InvalidBytecode("Truncated POP".into()));
                    }
                    let reg = bytecode[pc + 1] as usize;
                    if reg < 32 {
                        if let Some(val) = stack.pop() {
                            registers[reg] = val;
                        }
                    }
                    pc += 2;
                }

                // ADD r_dst, r_a, r_b
                0x10 => {
                    if pc + 3 >= bytecode.len() {
                        return Err(ExecutionError::InvalidBytecode("Truncated ADD".into()));
                    }
                    let (dst, a, b) = (
                        bytecode[pc + 1] as usize,
                        bytecode[pc + 2] as usize,
                        bytecode[pc + 3] as usize,
                    );
                    if dst < 32 && a < 32 && b < 32 {
                        registers[dst] = registers[a].wrapping_add(registers[b]);
                    }
                    pc += 4;
                }

                // SUB r_dst, r_a, r_b
                0x11 => {
                    if pc + 3 >= bytecode.len() {
                        return Err(ExecutionError::InvalidBytecode("Truncated SUB".into()));
                    }
                    let (dst, a, b) = (
                        bytecode[pc + 1] as usize,
                        bytecode[pc + 2] as usize,
                        bytecode[pc + 3] as usize,
                    );
                    if dst < 32 && a < 32 && b < 32 {
                        registers[dst] = registers[a].wrapping_sub(registers[b]);
                    }
                    pc += 4;
                }

                // MUL r_dst, r_a, r_b
                0x12 => {
                    if pc + 3 >= bytecode.len() {
                        return Err(ExecutionError::InvalidBytecode("Truncated MUL".into()));
                    }
                    let (dst, a, b) = (
                        bytecode[pc + 1] as usize,
                        bytecode[pc + 2] as usize,
                        bytecode[pc + 3] as usize,
                    );
                    if dst < 32 && a < 32 && b < 32 {
                        registers[dst] = registers[a].wrapping_mul(registers[b]);
                    }
                    pc += 4;
                }

                // EMIT_LOG topic_reg, data_reg
                0x50 => {
                    if pc + 2 >= bytecode.len() {
                        return Err(ExecutionError::InvalidBytecode("Truncated EMIT".into()));
                    }
                    let topic_reg = bytecode[pc + 1] as usize;
                    let data_reg = bytecode[pc + 2] as usize;

                    let mut topic = [0u8; 32];
                    if topic_reg < 32 {
                        topic[24..32].copy_from_slice(&registers[topic_reg].to_be_bytes());
                    }

                    let data = if data_reg < 32 {
                        registers[data_reg].to_le_bytes().to_vec()
                    } else {
                        Vec::new()
                    };

                    logs.push(ExecutionLog {
                        topics: vec![topic],
                        data,
                    });
                    pc += 3;
                    *gas_remaining = gas_remaining.saturating_sub(99); // Log costs more gas
                }

                // RET (return R0)
                0x70 => {
                    *output = registers[0].to_le_bytes().to_vec();
                    break;
                }

                // Unknown opcode - skip
                _ => pc += 1,
            }

            // Stack overflow check
            if stack.len() > self.config.stack_limit {
                return Err(ExecutionError::StackOverflow);
            }
        }

        // Default output from R0 if not set
        if output.is_empty() {
            *output = registers[0].to_le_bytes().to_vec();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_execution() {
        let config = VmConfig::default();
        let executor = X3Executor::new(config);

        // Minimal valid bytecode (magic + padding + halt)
        let bytecode = vec![
            0x58, 0x33, 0x56, 0x43, // Magic: X3VC
            0x00, 0x01, // Version
            0x00, 0x00, // Flags
            0x00, 0x00, 0x00, 0x00, // Padding
            0xA1, // HALT
        ];

        let result = executor.execute(&bytecode, &[], 1000).unwrap();
        assert!(result.success);
    }

    #[test]
    fn test_invalid_magic() {
        let config = VmConfig::default();
        let executor = X3Executor::new(config);

        let bytecode = vec![0x00, 0x00, 0x00, 0x00];
        let result = executor.execute(&bytecode, &[], 1000);
        assert!(result.is_err());
    }
}
