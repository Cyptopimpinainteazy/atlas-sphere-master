//! Project configuration for x3 CLI.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Default configuration file name.
pub const CONFIG_FILE: &str = "x3.toml";

/// Project configuration loaded from x3.toml.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Project name.
    pub name: String,

    /// Project version.
    pub version: String,

    /// Project type (evm, svm, dual).
    #[serde(default)]
    pub project_type: ProjectType,

    /// Network configuration.
    #[serde(default)]
    pub network: NetworkConfig,

    /// Build configuration.
    #[serde(default)]
    pub build: BuildConfig,

    /// Test configuration.
    #[serde(default)]
    pub test: TestConfig,

    /// Contract paths.
    #[serde(default)]
    pub contracts: ContractsConfig,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            name: "my-atlas-project".to_string(),
            version: "0.1.0".to_string(),
            project_type: ProjectType::Dual,
            network: NetworkConfig::default(),
            build: BuildConfig::default(),
            test: TestConfig::default(),
            contracts: ContractsConfig::default(),
        }
    }
}

impl ProjectConfig {
    /// Load configuration from file.
    pub fn load(path: impl AsRef<Path>) -> crate::error::Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to file.
    pub fn save(&self, path: impl AsRef<Path>) -> crate::error::Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path.as_ref(), content)?;
        Ok(())
    }

    /// Find configuration file in current or parent directories.
    pub fn find() -> Option<PathBuf> {
        let mut current = std::env::current_dir().ok()?;
        loop {
            let config_path = current.join(CONFIG_FILE);
            if config_path.exists() {
                return Some(config_path);
            }
            if !current.pop() {
                break;
            }
        }
        None
    }

    /// Load configuration from current directory or parents.
    pub fn load_from_current_dir() -> crate::error::Result<Self> {
        match Self::find() {
            Some(path) => Self::load(path),
            None => Err(crate::error::CliError::Config(
                "No x3.toml found in current directory or parents".to_string(),
            )),
        }
    }
}

/// Project type enumeration.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ProjectType {
    /// EVM-only project.
    Evm,
    /// SVM-only project.
    Svm,
    /// Dual-VM project (default).
    #[default]
    Dual,
}

/// Network configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Default network to use.
    #[serde(default = "default_network")]
    pub default: String,

    /// Local development endpoint.
    #[serde(default = "default_local_endpoint")]
    pub local: String,

    /// Testnet endpoint.
    #[serde(default = "default_testnet_endpoint")]
    pub testnet: String,

    /// Mainnet endpoint.
    #[serde(default = "default_mainnet_endpoint")]
    pub mainnet: String,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            default: default_network(),
            local: default_local_endpoint(),
            testnet: default_testnet_endpoint(),
            mainnet: default_mainnet_endpoint(),
        }
    }
}

impl NetworkConfig {
    /// Get endpoint for the specified network.
    pub fn get_endpoint(&self, network: Option<&str>) -> &str {
        match network.unwrap_or(&self.default) {
            "local" | "dev" => &self.local,
            "testnet" | "test" => &self.testnet,
            "mainnet" | "main" => &self.mainnet,
            _ => &self.local,
        }
    }
}

fn default_network() -> String {
    "local".to_string()
}

#[cfg(feature = "sdk")]
fn default_local_endpoint() -> String {
    atlas_sdk::DEFAULT_HTTP_ENDPOINT.to_string()
}

#[cfg(not(feature = "sdk"))]
fn default_local_endpoint() -> String {
    "http://localhost:9944".to_string()
}

#[cfg(feature = "sdk")]
fn default_testnet_endpoint() -> String {
    atlas_sdk::TESTNET_HTTP_ENDPOINT.to_string()
}

#[cfg(not(feature = "sdk"))]
fn default_testnet_endpoint() -> String {
    "http://rpc.testnet.atlas-sphere.io:9944".to_string()
}

#[cfg(feature = "sdk")]
fn default_mainnet_endpoint() -> String {
    atlas_sdk::MAINNET_HTTP_ENDPOINT.to_string()
}

#[cfg(not(feature = "sdk"))]
fn default_mainnet_endpoint() -> String {
    "http://rpc.atlas-sphere.io:9944".to_string()
}

/// Build configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BuildConfig {
    /// Optimization level (0-3).
    #[serde(default = "default_optimization")]
    pub optimization: u8,

    /// Output directory.
    #[serde(default = "default_out_dir")]
    pub out_dir: String,

    /// Enable source maps.
    #[serde(default)]
    pub source_maps: bool,

    /// EVM compiler (solc, vyper).
    #[serde(default = "default_evm_compiler")]
    pub evm_compiler: String,

    /// Solidity version.
    #[serde(default = "default_solc_version")]
    pub solc_version: String,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            optimization: default_optimization(),
            out_dir: default_out_dir(),
            source_maps: false,
            evm_compiler: default_evm_compiler(),
            solc_version: default_solc_version(),
        }
    }
}

fn default_optimization() -> u8 {
    3
}

fn default_out_dir() -> String {
    "out".to_string()
}

fn default_evm_compiler() -> String {
    "solc".to_string()
}

fn default_solc_version() -> String {
    "0.8.24".to_string()
}

/// Test configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestConfig {
    /// Test framework (forge, hardhat).
    #[serde(default = "default_test_framework")]
    pub framework: String,

    /// Verbose output.
    #[serde(default)]
    pub verbose: bool,

    /// Gas report.
    #[serde(default)]
    pub gas_report: bool,

    /// Fork URL for mainnet forking.
    #[serde(default)]
    pub fork_url: Option<String>,

    /// Fork block number.
    #[serde(default)]
    pub fork_block: Option<u64>,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            framework: default_test_framework(),
            verbose: false,
            gas_report: false,
            fork_url: None,
            fork_block: None,
        }
    }
}

fn default_test_framework() -> String {
    "forge".to_string()
}

/// Contracts configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContractsConfig {
    /// Source directory for EVM contracts.
    #[serde(default = "default_evm_src")]
    pub evm_src: String,

    /// Source directory for SVM programs.
    #[serde(default = "default_svm_src")]
    pub svm_src: String,

    /// Remappings for imports.
    #[serde(default)]
    pub remappings: Vec<String>,

    /// Libraries to link.
    #[serde(default)]
    pub libraries: Vec<String>,
}

impl Default for ContractsConfig {
    fn default() -> Self {
        Self {
            evm_src: default_evm_src(),
            svm_src: default_svm_src(),
            remappings: vec![],
            libraries: vec![],
        }
    }
}

fn default_evm_src() -> String {
    "contracts/evm".to_string()
}

fn default_svm_src() -> String {
    "contracts/svm".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ProjectConfig::default();
        assert_eq!(config.project_type, ProjectType::Dual);
        assert_eq!(config.network.default, "local");
    }

    #[test]
    fn test_config_serialization() {
        let config = ProjectConfig::default();
        let toml = toml::to_string_pretty(&config).unwrap();
        assert!(toml.contains("name"));
        assert!(toml.contains("version"));
    }

    #[test]
    fn test_network_endpoint() {
        let config = NetworkConfig::default();
        assert!(config.get_endpoint(Some("local")).contains("localhost"));
        assert!(config.get_endpoint(Some("testnet")).contains("testnet"));
    }
}
