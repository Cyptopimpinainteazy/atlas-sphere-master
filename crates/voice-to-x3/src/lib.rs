//! # Voice-to-X3 Compiler
//!
//! Natural language to X3 smart contract code generation.

#![allow(dead_code)]
#![allow(unused_variables)]

pub mod error;
pub mod generator;
pub mod intent;
pub mod templates;

pub use error::{VoiceError, VoiceResult};
pub use generator::{CodeGenerator, GeneratedContract};
pub use intent::{ContractType, Intent, IntentParser, ParamValue};
pub use templates::Templates;

/// Voice-to-X3 version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Main Voice-to-X3 compiler interface
pub struct VoiceCompiler {
    parser: IntentParser,
    generator: CodeGenerator,
}

impl VoiceCompiler {
    /// Create a new compiler instance
    pub fn new() -> Self {
        Self {
            parser: IntentParser::new(),
            generator: CodeGenerator::new(),
        }
    }

    /// Compile natural language description into X3 code
    pub fn compile(&self, description: &str) -> VoiceResult<GeneratedContract> {
        let intent = self.parser.parse(description)?;
        let contract = self.generator.generate(&intent)?;
        Ok(contract)
    }

    /// Get the parsed intent without generating code
    pub fn parse_intent(&self, description: &str) -> VoiceResult<Intent> {
        self.parser.parse(description)
    }
}

impl Default for VoiceCompiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_token() {
        let compiler = VoiceCompiler::new();
        let result = compiler.compile("Create a token called CoolToken with 1 million supply");
        assert!(result.is_ok());
        let contract = result.unwrap();
        assert_eq!(contract.contract_type, ContractType::Token);
        assert!(contract.code.contains("CoolToken"));
    }

    #[test]
    fn test_compile_nft() {
        let compiler = VoiceCompiler::new();
        let result = compiler.compile("Make an NFT collection with 10000 items and 5% royalty");
        assert!(result.is_ok());
        let contract = result.unwrap();
        assert_eq!(contract.contract_type, ContractType::NFT);
    }

    #[test]
    fn test_compile_dex() {
        let compiler = VoiceCompiler::new();
        let result = compiler.compile("Build a DEX with 0.3% trading fee");
        assert!(result.is_ok());
        let contract = result.unwrap();
        assert_eq!(contract.contract_type, ContractType::DEX);
    }
}
