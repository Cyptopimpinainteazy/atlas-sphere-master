//! Chromosome representation for X3 bytecode strategies

use crate::error::{EvolutionError, Result};
use serde::{Deserialize, Serialize};

/// A gene represents a single unit of genetic information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Gene {
    /// Gene type (parameter, opcode, etc.)
    pub gene_type: GeneType,
    /// Raw value
    pub value: u64,
    /// Constraints for mutation
    pub constraints: GeneConstraints,
}

/// Type of gene
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeneType {
    /// Numeric parameter (threshold, amount, etc.)
    Parameter,
    /// Opcode/instruction
    Opcode,
    /// Memory address/offset
    Address,
    /// Control flow (branch, jump)
    ControlFlow,
    /// Constant value
    Constant,
    /// Register reference
    Register,
}

/// Constraints on gene mutation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneConstraints {
    /// Minimum value
    pub min: u64,
    /// Maximum value
    pub max: u64,
    /// Whether this gene is mutable
    pub mutable: bool,
    /// Allowed values (if restricted)
    pub allowed: Option<Vec<u64>>,
}

impl Default for GeneConstraints {
    fn default() -> Self {
        Self {
            min: 0,
            max: u64::MAX,
            mutable: true,
            allowed: None,
        }
    }
}

impl Gene {
    /// Create a new parameter gene
    pub fn parameter(value: u64, min: u64, max: u64) -> Self {
        Self {
            gene_type: GeneType::Parameter,
            value,
            constraints: GeneConstraints {
                min,
                max,
                mutable: true,
                allowed: None,
            },
        }
    }

    /// Create a new opcode gene
    pub fn opcode(value: u64, allowed: Vec<u64>) -> Self {
        Self {
            gene_type: GeneType::Opcode,
            value,
            constraints: GeneConstraints {
                min: 0,
                max: 255,
                mutable: true,
                allowed: Some(allowed),
            },
        }
    }

    /// Create an immutable constant gene
    pub fn constant(value: u64) -> Self {
        Self {
            gene_type: GeneType::Constant,
            value,
            constraints: GeneConstraints {
                min: value,
                max: value,
                mutable: false,
                allowed: None,
            },
        }
    }

    /// Create a register gene
    pub fn register(reg: u64, max_reg: u64) -> Self {
        Self {
            gene_type: GeneType::Register,
            value: reg,
            constraints: GeneConstraints {
                min: 0,
                max: max_reg,
                mutable: true,
                allowed: None,
            },
        }
    }

    /// Check if gene can be mutated
    pub fn is_mutable(&self) -> bool {
        self.constraints.mutable
    }

    /// Mutate gene value within constraints
    pub fn mutate(&mut self, new_value: u64) -> bool {
        if !self.constraints.mutable {
            return false;
        }

        // Check allowed values
        if let Some(ref allowed) = self.constraints.allowed {
            if !allowed.contains(&new_value) {
                return false;
            }
        }

        // Check range
        if new_value < self.constraints.min || new_value > self.constraints.max {
            return false;
        }

        self.value = new_value;
        true
    }
}

/// Chromosome represents a complete strategy as a sequence of genes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chromosome {
    /// Genes making up the chromosome
    genes: Vec<Gene>,
    /// Original bytecode for reconstruction
    bytecode_template: Vec<u8>,
    /// Mapping from gene index to bytecode offset
    gene_offsets: Vec<usize>,
    /// Strategy metadata
    pub metadata: ChromosomeMetadata,
}

/// Metadata about the chromosome
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChromosomeMetadata {
    /// Original source (if known)
    pub source: Option<String>,
    /// Generation this chromosome was created
    pub generation: usize,
    /// Parent chromosome IDs
    pub parents: Vec<u64>,
    /// Unique ID
    pub id: u64,
}

impl Chromosome {
    /// Create chromosome from raw bytecode
    pub fn from_bytecode(bytecode: Vec<u8>) -> Result<Self> {
        let (genes, gene_offsets) = Self::parse_bytecode(&bytecode)?;

        Ok(Self {
            genes,
            bytecode_template: bytecode,
            gene_offsets,
            metadata: ChromosomeMetadata {
                id: rand::random(),
                ..Default::default()
            },
        })
    }

    /// Parse bytecode into genes
    fn parse_bytecode(bytecode: &[u8]) -> Result<(Vec<Gene>, Vec<usize>)> {
        let mut genes = Vec::new();
        let mut offsets = Vec::new();
        let mut offset = 0;

        // X3 bytecode format:
        // [magic:4][version:2][header:N][instructions...]
        // For short bytecode (tests), skip header validation

        if bytecode.is_empty() {
            return Err(EvolutionError::InvalidBytecode("Empty bytecode".into()));
        }

        // Simplified parsing: treat every 4 bytes as a potential instruction
        while offset + 4 <= bytecode.len() {
            let opcode = bytecode[offset];

            // Determine gene type based on opcode patterns
            let gene_type = match opcode {
                0x00..=0x1F => GeneType::ControlFlow, // Control flow ops
                0x20..=0x3F => GeneType::Parameter,   // Arithmetic ops (have params)
                0x40..=0x5F => GeneType::Address,     // Memory ops
                0x60..=0x7F => GeneType::Register,    // Register ops
                _ => GeneType::Opcode,                // Other
            };

            // Extract value based on type
            let value = if offset + 4 <= bytecode.len() {
                u32::from_le_bytes([
                    bytecode[offset],
                    bytecode.get(offset + 1).copied().unwrap_or(0),
                    bytecode.get(offset + 2).copied().unwrap_or(0),
                    bytecode.get(offset + 3).copied().unwrap_or(0),
                ]) as u64
            } else {
                bytecode[offset] as u64
            };

            let gene = match gene_type {
                GeneType::Parameter => Gene::parameter(value, 0, u32::MAX as u64),
                GeneType::Opcode => Gene {
                    gene_type: GeneType::Opcode,
                    value,
                    constraints: GeneConstraints::default(),
                },
                GeneType::Register => Gene::register(value & 0xFF, 31),
                _ => Gene {
                    gene_type,
                    value,
                    constraints: GeneConstraints::default(),
                },
            };

            genes.push(gene);
            offsets.push(offset);
            offset += 4;
        }

        Ok((genes, offsets))
    }

    /// Convert chromosome back to bytecode
    pub fn to_bytecode(&self) -> Vec<u8> {
        let mut bytecode = self.bytecode_template.clone();

        // Apply gene values to bytecode
        for (i, gene) in self.genes.iter().enumerate() {
            if i < self.gene_offsets.len() {
                let offset = self.gene_offsets[i];
                let value_bytes = (gene.value as u32).to_le_bytes();

                for (j, byte) in value_bytes.iter().enumerate() {
                    if offset + j < bytecode.len() {
                        bytecode[offset + j] = *byte;
                    }
                }
            }
        }

        bytecode
    }

    /// Get number of genes
    pub fn len(&self) -> usize {
        self.genes.len()
    }

    /// Check if chromosome is empty
    pub fn is_empty(&self) -> bool {
        self.genes.is_empty()
    }

    /// Get gene at index
    pub fn get(&self, index: usize) -> Option<&Gene> {
        self.genes.get(index)
    }

    /// Get mutable gene at index
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Gene> {
        self.genes.get_mut(index)
    }

    /// Get all genes
    pub fn genes(&self) -> &[Gene] {
        &self.genes
    }

    /// Get mutable genes
    pub fn genes_mut(&mut self) -> &mut [Gene] {
        &mut self.genes
    }

    /// Get indices of mutable genes
    pub fn mutable_indices(&self) -> Vec<usize> {
        self.genes
            .iter()
            .enumerate()
            .filter(|(_, g)| g.is_mutable())
            .map(|(i, _)| i)
            .collect()
    }

    /// Get genes of a specific type
    pub fn genes_of_type(&self, gene_type: GeneType) -> Vec<(usize, &Gene)> {
        self.genes
            .iter()
            .enumerate()
            .filter(|(_, g)| g.gene_type == gene_type)
            .collect()
    }

    /// Compute hash of chromosome
    pub fn hash(&self) -> [u8; 32] {
        blake3::hash(&self.to_bytecode()).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gene_parameter() {
        let mut gene = Gene::parameter(100, 0, 200);
        assert!(gene.is_mutable());
        assert!(gene.mutate(150));
        assert_eq!(gene.value, 150);
        assert!(!gene.mutate(250)); // Out of range
    }

    #[test]
    fn test_gene_constant() {
        let mut gene = Gene::constant(42);
        assert!(!gene.is_mutable());
        assert!(!gene.mutate(100));
        assert_eq!(gene.value, 42);
    }

    #[test]
    fn test_chromosome_from_bytecode() {
        let bytecode = vec![0x20, 0x01, 0x02, 0x03, 0x40, 0x05, 0x06, 0x07];
        let chromosome = Chromosome::from_bytecode(bytecode.clone()).unwrap();
        assert!(!chromosome.is_empty());

        // Should reconstruct to same bytecode
        let reconstructed = chromosome.to_bytecode();
        assert_eq!(reconstructed, bytecode);
    }

    #[test]
    fn test_chromosome_mutation() {
        let bytecode = vec![0x20, 0x64, 0x00, 0x00, 0x20, 0xC8, 0x00, 0x00];
        let mut chromosome = Chromosome::from_bytecode(bytecode).unwrap();

        // Mutate a parameter gene
        let mutable = chromosome.mutable_indices();
        assert!(!mutable.is_empty());

        if let Some(gene) = chromosome.get_mut(mutable[0]) {
            gene.mutate(0x12345678);
        }

        let new_bytecode = chromosome.to_bytecode();
        // Bytecode should be different after mutation
        assert_ne!(new_bytecode[1..4], [0x64, 0x00, 0x00]);
    }
}
