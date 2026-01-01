// crates/x3-runtime/src/validation.rs
// Bytecode validation for X3 compiled WASM
//
// Security checks before execution:
// - WASM structure validation
// - Size limits
// - Dangerous opcode detection
// - Gas estimation

use anyhow::{anyhow, Result};
use std::collections::HashSet;

/// WASM bytecode validator
pub struct WasmValidator {
    /// Maximum module size (bytes)
    max_size: usize,
    /// Maximum function count
    max_functions: u32,
    /// Maximum import count
    max_imports: u32,
    /// Maximum memory pages
    max_memory_pages: u32,
    /// Banned opcodes
    banned_opcodes: HashSet<u8>,
}

impl WasmValidator {
    pub fn new() -> Self {
        let mut banned = HashSet::new();
        // No banned opcodes by default
        // Could add: floating point, SIMD, etc.
        
        WasmValidator {
            max_size: 16384, // 16KB
            max_functions: 256,
            max_imports: 64,
            max_memory_pages: 4, // 256KB max memory
            banned_opcodes: banned,
        }
    }

    /// Strict validator for production
    pub fn strict() -> Self {
        let mut banned = HashSet::new();
        
        // Ban floating point opcodes
        banned.insert(0x43); // f32.const
        banned.insert(0x44); // f64.const
        banned.insert(0x91); // f32.abs
        banned.insert(0x92); // f32.neg
        banned.insert(0x99); // f64.abs
        banned.insert(0x9a); // f64.neg
        
        // Ban SIMD (0xfd prefix)
        // Handled separately
        
        WasmValidator {
            max_size: 16384,
            max_functions: 128,
            max_imports: 32,
            max_memory_pages: 2, // 128KB
            banned_opcodes: banned,
        }
    }

    /// Validate WASM bytecode
    pub fn validate(&self, wasm: &[u8]) -> Result<ValidationResult> {
        let mut result = ValidationResult::new();
        
        // Size check
        if wasm.len() > self.max_size {
            return Err(anyhow!("WASM too large: {} > {}", wasm.len(), self.max_size));
        }
        result.size = wasm.len();
        
        // Magic number check
        if wasm.len() < 8 {
            return Err(anyhow!("WASM too short"));
        }
        if &wasm[0..4] != b"\0asm" {
            return Err(anyhow!("Invalid WASM magic"));
        }
        
        // Version check
        let version = u32::from_le_bytes([wasm[4], wasm[5], wasm[6], wasm[7]]);
        if version != 1 {
            return Err(anyhow!("Unsupported WASM version: {}", version));
        }
        result.version = version;
        
        // Parse sections
        let mut pos = 8;
        while pos < wasm.len() {
            let section_id = wasm[pos];
            pos += 1;
            
            let (size, bytes_read) = self.read_leb128(&wasm[pos..])?;
            pos += bytes_read;
            
            if pos + size as usize > wasm.len() {
                return Err(anyhow!("Section exceeds WASM bounds"));
            }
            
            let section_data = &wasm[pos..pos + size as usize];
            pos += size as usize;
            
            match section_id {
                1 => self.validate_type_section(section_data, &mut result)?,
                2 => self.validate_import_section(section_data, &mut result)?,
                3 => self.validate_function_section(section_data, &mut result)?,
                5 => self.validate_memory_section(section_data, &mut result)?,
                7 => self.validate_export_section(section_data, &mut result)?,
                10 => self.validate_code_section(section_data, &mut result)?,
                _ => {} // Ignore other sections
            }
        }
        
        // Check limits
        if result.function_count > self.max_functions {
            return Err(anyhow!("Too many functions: {} > {}", 
                result.function_count, self.max_functions));
        }
        if result.import_count > self.max_imports {
            return Err(anyhow!("Too many imports: {} > {}",
                result.import_count, self.max_imports));
        }
        if result.memory_pages > self.max_memory_pages {
            return Err(anyhow!("Too much memory: {} pages > {}",
                result.memory_pages, self.max_memory_pages));
        }
        
        result.valid = true;
        Ok(result)
    }

    fn read_leb128(&self, data: &[u8]) -> Result<(u32, usize)> {
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

    fn validate_type_section(&self, data: &[u8], result: &mut ValidationResult) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }
        
        let (count, _) = self.read_leb128(data)?;
        result.type_count = count;
        
        Ok(())
    }

    fn validate_import_section(&self, data: &[u8], result: &mut ValidationResult) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }
        
        let (count, mut pos) = self.read_leb128(data)?;
        result.import_count = count;
        
        // Parse each import to collect names
        for _ in 0..count {
            // Module name
            let (mod_len, bytes) = self.read_leb128(&data[pos..])?;
            pos += bytes;
            let module = std::str::from_utf8(&data[pos..pos + mod_len as usize])
                .unwrap_or("?").to_string();
            pos += mod_len as usize;
            
            // Function name
            let (name_len, bytes) = self.read_leb128(&data[pos..])?;
            pos += bytes;
            let name = std::str::from_utf8(&data[pos..pos + name_len as usize])
                .unwrap_or("?").to_string();
            pos += name_len as usize;
            
            result.imports.push(format!("{}::{}", module, name));
            
            // Import kind
            let kind = data[pos];
            pos += 1;
            
            // Type index or other descriptor
            let (_, bytes) = self.read_leb128(&data[pos..])?;
            pos += bytes;
            
            // Memory imports have additional fields
            if kind == 2 {
                let (_, bytes) = self.read_leb128(&data[pos..])?;
                pos += bytes;
            }
        }
        
        Ok(())
    }

    fn validate_function_section(&self, data: &[u8], result: &mut ValidationResult) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }
        
        let (count, _) = self.read_leb128(data)?;
        result.function_count = count;
        
        Ok(())
    }

    fn validate_memory_section(&self, data: &[u8], result: &mut ValidationResult) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }
        
        let (count, mut pos) = self.read_leb128(data)?;
        
        for _ in 0..count {
            let flags = data[pos];
            pos += 1;
            
            // Initial pages
            let (initial, bytes) = self.read_leb128(&data[pos..])?;
            pos += bytes;
            
            result.memory_pages = result.memory_pages.max(initial);
            
            // Maximum pages (if flag set)
            if flags & 1 != 0 {
                let (max, bytes) = self.read_leb128(&data[pos..])?;
                pos += bytes;
                result.memory_pages = result.memory_pages.max(max);
            }
        }
        
        Ok(())
    }

    fn validate_export_section(&self, data: &[u8], result: &mut ValidationResult) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }
        
        let (count, mut pos) = self.read_leb128(data)?;
        result.export_count = count;
        
        for _ in 0..count {
            let (name_len, bytes) = self.read_leb128(&data[pos..])?;
            pos += bytes;
            
            let name = std::str::from_utf8(&data[pos..pos + name_len as usize])
                .unwrap_or("?").to_string();
            pos += name_len as usize;
            
            let kind = data[pos];
            pos += 1;
            
            let (_, bytes) = self.read_leb128(&data[pos..])?;
            pos += bytes;
            
            // Only track function exports
            if kind == 0 {
                result.exports.push(name);
            }
        }
        
        Ok(())
    }

    fn validate_code_section(&self, data: &[u8], result: &mut ValidationResult) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }
        
        let (count, mut pos) = self.read_leb128(data)?;
        
        for _ in 0..count {
            let (body_size, bytes) = self.read_leb128(&data[pos..])?;
            pos += bytes;
            
            // Scan for banned opcodes
            let body_end = pos + body_size as usize;
            while pos < body_end {
                let opcode = data[pos];
                
                if self.banned_opcodes.contains(&opcode) {
                    result.banned_opcodes.push(opcode);
                }
                
                // Estimate gas based on opcode
                result.estimated_gas += self.opcode_gas_cost(opcode);
                
                pos += 1;
                
                // Skip operands for known opcodes
                pos += self.skip_operands(opcode, &data[pos..]);
            }
        }
        
        if !result.banned_opcodes.is_empty() {
            return Err(anyhow!("Banned opcodes detected: {:?}", result.banned_opcodes));
        }
        
        Ok(())
    }

    fn opcode_gas_cost(&self, opcode: u8) -> u64 {
        match opcode {
            // Control flow
            0x00 => 0,   // unreachable
            0x01 => 1,   // nop
            0x02..=0x04 => 5,  // block, loop, if
            0x05 => 3,   // else
            0x0b => 2,   // end
            0x0c..=0x0e => 8,  // br, br_if, br_table
            0x0f => 5,   // return
            0x10 => 100, // call
            0x11 => 150, // call_indirect
            
            // Stack
            0x1a => 1,   // drop
            0x1b => 2,   // select
            
            // Locals/globals
            0x20..=0x24 => 3,
            
            // Memory
            0x28..=0x3e => 50, // loads/stores
            0x3f => 1000,     // memory.size
            0x40 => 10000,    // memory.grow
            
            // Constants
            0x41..=0x44 => 1,
            
            // Comparisons
            0x45..=0x66 => 3,
            
            // Arithmetic
            0x67..=0x8a => 5,
            
            // Conversions
            0xa7..=0xbf => 3,
            
            // Default
            _ => 10,
        }
    }

    fn skip_operands(&self, opcode: u8, data: &[u8]) -> usize {
        match opcode {
            // Fixed-size immediates
            0x41 => 1, // i32.const (LEB128, estimate 1)
            0x42 => 1, // i64.const
            0x43 => 4, // f32.const
            0x44 => 8, // f64.const
            
            // Block types
            0x02..=0x04 => 1,
            
            // Branch indices
            0x0c | 0x0d => 1,
            
            // Call indices
            0x10 | 0x11 => 1,
            
            // Local/global indices
            0x20..=0x24 => 1,
            
            // Memory operations (align + offset)
            0x28..=0x3e => 2,
            
            _ => 0,
        }
    }
}

impl Default for WasmValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub valid: bool,
    pub size: usize,
    pub version: u32,
    pub type_count: u32,
    pub function_count: u32,
    pub import_count: u32,
    pub export_count: u32,
    pub memory_pages: u32,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
    pub banned_opcodes: Vec<u8>,
    pub estimated_gas: u64,
}

impl ValidationResult {
    pub fn new() -> Self {
        ValidationResult {
            valid: false,
            size: 0,
            version: 0,
            type_count: 0,
            function_count: 0,
            import_count: 0,
            export_count: 0,
            memory_pages: 0,
            imports: Vec::new(),
            exports: Vec::new(),
            banned_opcodes: Vec::new(),
            estimated_gas: 0,
        }
    }
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Gas estimator for X3 operations
pub struct GasEstimator;

impl GasEstimator {
    /// Estimate gas for WASM execution
    pub fn estimate_wasm(wasm: &[u8]) -> Result<u64> {
        let validator = WasmValidator::new();
        let result = validator.validate(wasm)?;
        
        // Base cost + estimated instruction cost
        Ok(21000 + result.estimated_gas)
    }

    /// Estimate gas for token transfer
    pub fn estimate_transfer() -> u64 {
        65000
    }

    /// Estimate gas for token approval
    pub fn estimate_approval() -> u64 {
        50000
    }

    /// Estimate gas for DEX swap
    pub fn estimate_swap(hop_count: usize) -> u64 {
        150000 + (hop_count as u64 * 50000)
    }

    /// Estimate gas for vault deposit
    pub fn estimate_vault_deposit() -> u64 {
        100000
    }

    /// Estimate gas for flashloan
    pub fn estimate_flashloan() -> u64 {
        500000
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_wasm() -> Vec<u8> {
        vec![
            0x00, 0x61, 0x73, 0x6d,  // \0asm
            0x01, 0x00, 0x00, 0x00,  // version 1
            0x01, 0x04,              // type section
            0x01, 0x60, 0x00, 0x00,  // 1 func type () -> ()
            0x03, 0x02,              // function section
            0x01, 0x00,              // 1 function, type 0
            0x07, 0x08,              // export section
            0x01, 0x04, b'm', b'a', b'i', b'n', 0x00, 0x00,
            0x0a, 0x04,              // code section
            0x01, 0x02, 0x00, 0x0b,  // 1 body, size 2, 0 locals, end
        ]
    }

    #[test]
    fn test_validator_creation() {
        let validator = WasmValidator::new();
        assert_eq!(validator.max_size, 16384);
        assert_eq!(validator.max_functions, 256);
    }

    #[test]
    fn test_strict_validator() {
        let validator = WasmValidator::strict();
        assert!(validator.banned_opcodes.contains(&0x43)); // f32.const banned
        assert!(validator.banned_opcodes.contains(&0x44)); // f64.const banned
    }

    #[test]
    fn test_validate_minimal() {
        let validator = WasmValidator::new();
        let wasm = minimal_wasm();
        
        let result = validator.validate(&wasm).unwrap();
        
        assert!(result.valid);
        assert_eq!(result.version, 1);
        assert_eq!(result.function_count, 1);
        assert!(result.exports.contains(&"main".to_string()));
    }

    #[test]
    fn test_validate_too_large() {
        let validator = WasmValidator::new();
        let wasm = vec![0u8; 20000]; // Larger than max
        
        assert!(validator.validate(&wasm).is_err());
    }

    #[test]
    fn test_validate_invalid_magic() {
        let validator = WasmValidator::new();
        let wasm = vec![0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00];
        
        assert!(validator.validate(&wasm).is_err());
    }

    #[test]
    fn test_gas_estimation() {
        let wasm = minimal_wasm();
        let gas = GasEstimator::estimate_wasm(&wasm).unwrap();
        
        // Should have base cost + some instruction cost
        assert!(gas >= 21000);
    }

    #[test]
    fn test_operation_gas() {
        assert_eq!(GasEstimator::estimate_transfer(), 65000);
        assert_eq!(GasEstimator::estimate_approval(), 50000);
        assert_eq!(GasEstimator::estimate_swap(1), 200000);
        assert_eq!(GasEstimator::estimate_swap(2), 250000);
    }
}
