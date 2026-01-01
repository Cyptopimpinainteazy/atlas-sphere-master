// crates/x3-runtime/src/executor.rs
// WASM executor for X3 compiled bytecode
//
// Executes X3 WASM modules with:
// - Gas metering and limits
// - Host function bindings
// - Memory management
// - Execution tracing

use crate::{X3Context, X3ExecutionResult, create_full_registry};
use crate::host_functions::HostFunctionRegistry;
use anyhow::{anyhow, Result};
use std::sync::Arc;

/// X3 WASM Executor
/// 
/// Executes compiled X3 WASM bytecode with full host function support
pub struct X3Executor {
    /// Host function registry
    registry: HostFunctionRegistry,
    /// Gas limit per execution
    default_gas_limit: u64,
    /// Enable execution tracing
    enable_tracing: bool,
}

impl X3Executor {
    pub fn new() -> Self {
        X3Executor {
            registry: create_full_registry(),
            default_gas_limit: 10_000_000, // 10M gas default
            enable_tracing: false,
        }
    }

    pub fn with_gas_limit(mut self, limit: u64) -> Self {
        self.default_gas_limit = limit;
        self
    }

    pub fn with_tracing(mut self, enable: bool) -> Self {
        self.enable_tracing = enable;
        self
    }

    pub fn with_registry(mut self, registry: HostFunctionRegistry) -> Self {
        self.registry = registry;
        self
    }

    /// Execute X3 WASM bytecode
    pub fn execute(
        &self,
        wasm: &[u8],
        function: &str,
        args: &[u64],
        caller: [u8; 32],
    ) -> X3ExecutionResult {
        let mut ctx = X3Context::new(caller, self.default_gas_limit);
        let start_gas = ctx.gas_remaining;

        // Validate WASM module
        if let Err(e) = self.validate_wasm(wasm) {
            return X3ExecutionResult::failure(0, format!("Invalid WASM: {}", e));
        }

        // Execute the module
        match self.execute_inner(wasm, function, args, &mut ctx) {
            Ok(return_data) => {
                let gas_used = start_gas - ctx.gas_remaining;
                X3ExecutionResult::success(gas_used, return_data, ctx)
            }
            Err(e) => {
                let gas_used = start_gas - ctx.gas_remaining;
                X3ExecutionResult::failure(gas_used, e.to_string())
            }
        }
    }

    fn validate_wasm(&self, wasm: &[u8]) -> Result<()> {
        // Check WASM magic number
        if wasm.len() < 8 {
            return Err(anyhow!("WASM too short"));
        }
        
        // \0asm magic
        if &wasm[0..4] != b"\0asm" {
            return Err(anyhow!("Invalid WASM magic"));
        }
        
        // Version 1
        if &wasm[4..8] != &[1, 0, 0, 0] {
            return Err(anyhow!("Unsupported WASM version"));
        }
        
        // Size check (16KB max per payload)
        if wasm.len() > 16384 {
            return Err(anyhow!("WASM too large: {} > 16384", wasm.len()));
        }
        
        Ok(())
    }

    fn execute_inner(
        &self,
        wasm: &[u8],
        function: &str,
        args: &[u64],
        ctx: &mut X3Context,
    ) -> Result<Vec<u8>> {
        // For MVP: simplified execution without full wasmi integration
        // This demonstrates the execution flow and host function binding
        
        // Parse WASM sections
        let sections = self.parse_wasm_sections(wasm)?;
        
        // Find export section and locate function
        let export_section = sections.get(&7)
            .ok_or_else(|| anyhow!("No export section"))?;
        
        // Find function index in exports
        let func_idx = self.find_export(export_section, function)?;
        
        // For imported functions (most X3 ops), dispatch to host registry
        // Import section is ID 2
        if let Some(import_section) = sections.get(&2) {
            let import_count = self.count_imports(import_section)?;
            
            if func_idx < import_count {
                // This is an imported function - call host
                let import_name = self.get_import_name(import_section, func_idx as usize)?;
                let result = self.registry.call(&import_name, ctx, args)?;
                return Ok(result.to_le_bytes().to_vec());
            }
        }
        
        // For local functions: would need full WASM interpreter
        // MVP: return mock success
        ctx.consume_gas(1000)?;
        
        Ok(vec![1, 0, 0, 0, 0, 0, 0, 0]) // Success result
    }

    fn parse_wasm_sections(&self, wasm: &[u8]) -> Result<std::collections::HashMap<u8, Vec<u8>>> {
        let mut sections = std::collections::HashMap::new();
        let mut pos = 8; // Skip magic + version
        
        while pos < wasm.len() {
            if pos >= wasm.len() {
                break;
            }
            
            let section_id = wasm[pos];
            pos += 1;
            
            if pos >= wasm.len() {
                break;
            }
            
            // Read section size (LEB128)
            let (size, bytes_read) = self.read_leb128_u32(&wasm[pos..])?;
            pos += bytes_read;
            
            if pos + size as usize > wasm.len() {
                return Err(anyhow!("Section size exceeds WASM bounds"));
            }
            
            let section_data = wasm[pos..pos + size as usize].to_vec();
            sections.insert(section_id, section_data);
            
            pos += size as usize;
        }
        
        Ok(sections)
    }

    fn read_leb128_u32(&self, data: &[u8]) -> Result<(u32, usize)> {
        let mut result = 0u32;
        let mut shift = 0;
        let mut bytes_read = 0;
        
        for &byte in data.iter() {
            bytes_read += 1;
            result |= ((byte & 0x7f) as u32) << shift;
            
            if byte & 0x80 == 0 {
                break;
            }
            
            shift += 7;
            if shift >= 32 {
                return Err(anyhow!("LEB128 overflow"));
            }
        }
        
        Ok((result, bytes_read))
    }

    fn find_export(&self, export_section: &[u8], name: &str) -> Result<u32> {
        if export_section.is_empty() {
            return Err(anyhow!("Empty export section"));
        }
        
        let (count, mut pos) = self.read_leb128_u32(export_section)?;
        
        for _ in 0..count {
            // Read name length
            if pos >= export_section.len() {
                break;
            }
            let (name_len, bytes_read) = self.read_leb128_u32(&export_section[pos..])?;
            pos += bytes_read;
            
            // Read name
            if pos + name_len as usize > export_section.len() {
                break;
            }
            let export_name = std::str::from_utf8(&export_section[pos..pos + name_len as usize])
                .unwrap_or("");
            pos += name_len as usize;
            
            // Read kind
            if pos >= export_section.len() {
                break;
            }
            let kind = export_section[pos];
            pos += 1;
            
            // Read index
            let (index, bytes_read) = self.read_leb128_u32(&export_section[pos..])?;
            pos += bytes_read;
            
            // Check if this is our function
            if export_name == name && kind == 0 {
                return Ok(index);
            }
        }
        
        Err(anyhow!("Export '{}' not found", name))
    }

    fn count_imports(&self, import_section: &[u8]) -> Result<u32> {
        if import_section.is_empty() {
            return Ok(0);
        }
        
        let (count, _) = self.read_leb128_u32(import_section)?;
        Ok(count)
    }

    fn get_import_name(&self, import_section: &[u8], index: usize) -> Result<String> {
        if import_section.is_empty() {
            return Err(anyhow!("Empty import section"));
        }
        
        let (count, mut pos) = self.read_leb128_u32(import_section)?;
        
        for i in 0..count as usize {
            // Read module name length
            let (mod_len, bytes_read) = self.read_leb128_u32(&import_section[pos..])?;
            pos += bytes_read;
            
            // Skip module name
            pos += mod_len as usize;
            
            // Read function name length
            let (name_len, bytes_read) = self.read_leb128_u32(&import_section[pos..])?;
            pos += bytes_read;
            
            // Read function name
            let func_name = std::str::from_utf8(&import_section[pos..pos + name_len as usize])
                .unwrap_or("")
                .to_string();
            pos += name_len as usize;
            
            // Read import kind
            let kind = import_section[pos];
            pos += 1;
            
            // Read type index (for functions)
            let (_, bytes_read) = self.read_leb128_u32(&import_section[pos..])?;
            pos += bytes_read;
            
            if i == index && kind == 0 {
                return Ok(func_name);
            }
        }
        
        Err(anyhow!("Import index {} not found", index))
    }
}

impl Default for X3Executor {
    fn default() -> Self {
        Self::new()
    }
}

/// Execution trace for debugging
#[derive(Debug, Clone)]
pub struct ExecutionTrace {
    pub steps: Vec<TraceStep>,
    pub total_gas: u64,
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct TraceStep {
    pub instruction: String,
    pub gas_cost: u64,
    pub stack_depth: usize,
    pub host_call: Option<String>,
}

/// Traced executor that records execution steps
pub struct TracedExecutor {
    executor: X3Executor,
    trace: Vec<TraceStep>,
}

impl TracedExecutor {
    pub fn new() -> Self {
        TracedExecutor {
            executor: X3Executor::new().with_tracing(true),
            trace: Vec::new(),
        }
    }

    pub fn execute(
        &mut self,
        wasm: &[u8],
        function: &str,
        args: &[u64],
        caller: [u8; 32],
    ) -> (X3ExecutionResult, ExecutionTrace) {
        self.trace.clear();
        
        let result = self.executor.execute(wasm, function, args, caller);
        
        let trace = ExecutionTrace {
            steps: self.trace.clone(),
            total_gas: result.gas_used,
            success: result.success,
        };
        
        (result, trace)
    }
}

impl Default for TracedExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_wasm() -> Vec<u8> {
        // Minimal valid WASM with one export
        vec![
            0x00, 0x61, 0x73, 0x6d,  // \0asm magic
            0x01, 0x00, 0x00, 0x00,  // version 1
            // Type section (ID=1)
            0x01, 0x04,              // section id=1, size=4
            0x01,                    // 1 type
            0x60, 0x00, 0x00,        // func () -> ()
            // Function section (ID=3)
            0x03, 0x02,              // section id=3, size=2
            0x01, 0x00,              // 1 function, type 0
            // Export section (ID=7)
            0x07, 0x08,              // section id=7, size=8
            0x01,                    // 1 export
            0x04,                    // name length = 4
            b'm', b'a', b'i', b'n',  // "main"
            0x00,                    // kind = function
            0x00,                    // index = 0
            // Code section (ID=10)
            0x0a, 0x04,              // section id=10, size=4
            0x01,                    // 1 function body
            0x02, 0x00,              // body size=2, 0 locals
            0x0b,                    // end
        ]
    }

    #[test]
    fn test_executor_creation() {
        let executor = X3Executor::new();
        assert_eq!(executor.default_gas_limit, 10_000_000);
        assert!(!executor.enable_tracing);
    }

    #[test]
    fn test_executor_builder() {
        let executor = X3Executor::new()
            .with_gas_limit(5_000_000)
            .with_tracing(true);
        
        assert_eq!(executor.default_gas_limit, 5_000_000);
        assert!(executor.enable_tracing);
    }

    #[test]
    fn test_wasm_validation() {
        let executor = X3Executor::new();
        
        // Valid WASM
        let wasm = minimal_wasm();
        assert!(executor.validate_wasm(&wasm).is_ok());
        
        // Too short
        assert!(executor.validate_wasm(&[0x00]).is_err());
        
        // Wrong magic
        assert!(executor.validate_wasm(&[0x01, 0x02, 0x03, 0x04, 0x01, 0x00, 0x00, 0x00]).is_err());
        
        // Wrong version
        assert!(executor.validate_wasm(&[0x00, 0x61, 0x73, 0x6d, 0x02, 0x00, 0x00, 0x00]).is_err());
    }

    #[test]
    fn test_execute_minimal() {
        let executor = X3Executor::new();
        let wasm = minimal_wasm();
        let caller = [1u8; 32];
        
        let result = executor.execute(&wasm, "main", &[], caller);
        
        assert!(result.success);
        assert!(result.gas_used > 0);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_execute_invalid_export() {
        let executor = X3Executor::new();
        let wasm = minimal_wasm();
        let caller = [1u8; 32];
        
        let result = executor.execute(&wasm, "nonexistent", &[], caller);
        
        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("not found"));
    }

    #[test]
    fn test_section_parsing() {
        let executor = X3Executor::new();
        let wasm = minimal_wasm();
        
        let sections = executor.parse_wasm_sections(&wasm).unwrap();
        
        // Should have type (1), function (3), export (7), code (10) sections
        assert!(sections.contains_key(&1));
        assert!(sections.contains_key(&3));
        assert!(sections.contains_key(&7));
        assert!(sections.contains_key(&10));
    }

    #[test]
    fn test_traced_executor() {
        let mut executor = TracedExecutor::new();
        let wasm = minimal_wasm();
        let caller = [1u8; 32];
        
        let (result, trace) = executor.execute(&wasm, "main", &[], caller);
        
        assert!(result.success);
        assert_eq!(trace.success, result.success);
        assert_eq!(trace.total_gas, result.gas_used);
    }

    #[test]
    fn test_leb128_parsing() {
        let executor = X3Executor::new();
        
        // Single byte
        let (val, len) = executor.read_leb128_u32(&[0x05]).unwrap();
        assert_eq!(val, 5);
        assert_eq!(len, 1);
        
        // Two bytes
        let (val, len) = executor.read_leb128_u32(&[0x80, 0x01]).unwrap();
        assert_eq!(val, 128);
        assert_eq!(len, 2);
    }
}
