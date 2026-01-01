//! Project management utilities.

use crate::config::{ProjectConfig, ProjectType};
use crate::error::{CliError, Result};
use std::path::{Path, PathBuf};

/// Project structure.
pub struct Project {
    /// Project root directory.
    pub root: PathBuf,
    /// Project configuration.
    pub config: ProjectConfig,
}

impl Project {
    /// Load project from directory.
    pub fn load(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let config_path = root.join(crate::config::CONFIG_FILE);

        if !config_path.exists() {
            return Err(CliError::Project(format!(
                "No x3.toml found in {}",
                root.display()
            )));
        }

        let config = ProjectConfig::load(&config_path)?;
        Ok(Self { root, config })
    }

    /// Load project from current directory.
    pub fn load_current() -> Result<Self> {
        let current = std::env::current_dir()?;
        Self::load(current)
    }

    /// Initialize a new project.
    pub fn init(root: impl AsRef<Path>, name: &str, project_type: ProjectType) -> Result<Self> {
        let root = root.as_ref().to_path_buf();

        // Create root directory if needed
        std::fs::create_dir_all(&root)?;

        // Check if already initialized
        let config_path = root.join(crate::config::CONFIG_FILE);
        if config_path.exists() {
            return Err(CliError::Project("Project already initialized".to_string()));
        }

        // Create configuration
        let config = ProjectConfig {
            name: name.to_string(),
            project_type: project_type.clone(),
            ..Default::default()
        };

        // Save configuration
        config.save(&config_path)?;

        // Create directory structure
        Self::create_structure(&root, &project_type)?;

        Ok(Self { root, config })
    }

    /// Create project directory structure.
    fn create_structure(root: &Path, project_type: &ProjectType) -> Result<()> {
        // Common directories
        let dirs = vec!["out", "test", "scripts", "docs"];

        for dir in dirs {
            std::fs::create_dir_all(root.join(dir))?;
        }

        // Type-specific directories
        match project_type {
            ProjectType::Evm => {
                std::fs::create_dir_all(root.join("contracts/evm"))?;
                std::fs::create_dir_all(root.join("lib"))?;
            }
            ProjectType::Svm => {
                std::fs::create_dir_all(root.join("contracts/svm"))?;
                std::fs::create_dir_all(root.join("programs"))?;
            }
            ProjectType::Dual => {
                std::fs::create_dir_all(root.join("contracts/evm"))?;
                std::fs::create_dir_all(root.join("contracts/svm"))?;
                std::fs::create_dir_all(root.join("lib"))?;
                std::fs::create_dir_all(root.join("programs"))?;
            }
        }

        // Create .gitignore
        let gitignore = r#"# Build artifacts
out/
cache/
artifacts/
node_modules/

# Environment
.env
.env.local

# IDE
.vscode/
.idea/

# OS
.DS_Store
Thumbs.db

# Rust
target/
Cargo.lock

# Solana
.anchor/
"#;
        std::fs::write(root.join(".gitignore"), gitignore)?;

        // Create README
        let readme = format!(
            "# {}\n\nAn Atlas Sphere project.\n\n## Getting Started\n\n```bash\nx3 build\nx3 test\nx3 deploy --network testnet\n```\n",
            root.file_name().unwrap_or_default().to_string_lossy()
        );
        std::fs::write(root.join("README.md"), readme)?;

        Ok(())
    }

    /// Get EVM contracts source directory.
    pub fn evm_src(&self) -> PathBuf {
        self.root.join(&self.config.contracts.evm_src)
    }

    /// Get SVM programs source directory.
    pub fn svm_src(&self) -> PathBuf {
        self.root.join(&self.config.contracts.svm_src)
    }

    /// Get output directory.
    pub fn out_dir(&self) -> PathBuf {
        self.root.join(&self.config.build.out_dir)
    }

    /// Find all Solidity files in the project.
    pub fn find_solidity_files(&self) -> Result<Vec<PathBuf>> {
        let src = self.evm_src();
        if !src.exists() {
            return Ok(vec![]);
        }

        let mut files = vec![];
        for entry in walkdir::WalkDir::new(&src) {
            let entry = entry?;
            if entry.path().extension().map_or(false, |e| e == "sol") {
                files.push(entry.path().to_path_buf());
            }
        }
        Ok(files)
    }

    /// Find all Rust files in SVM programs.
    pub fn find_svm_files(&self) -> Result<Vec<PathBuf>> {
        let src = self.svm_src();
        if !src.exists() {
            return Ok(vec![]);
        }

        let mut files = vec![];
        for entry in walkdir::WalkDir::new(&src) {
            let entry = entry?;
            if entry.path().extension().map_or(false, |e| e == "rs") {
                files.push(entry.path().to_path_buf());
            }
        }
        Ok(files)
    }
}

impl From<walkdir::Error> for CliError {
    fn from(err: walkdir::Error) -> Self {
        CliError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            err.to_string(),
        ))
    }
}
