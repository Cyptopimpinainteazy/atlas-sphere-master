//! Initialize command.

use crate::config::ProjectType;
use crate::error::Result;
use crate::project::Project;
use clap::Args;
use colored::Colorize;

#[derive(Args)]
pub struct InitArgs {
    /// Project name (defaults to directory name)
    #[arg(short, long)]
    pub name: Option<String>,

    /// Project type
    #[arg(short = 't', long, default_value = "dual")]
    pub project_type: String,

    /// Directory to initialize (defaults to current)
    #[arg(default_value = ".")]
    pub path: String,
}

pub async fn execute(args: InitArgs) -> Result<()> {
    let path = std::path::Path::new(&args.path);
    let name = args.name.unwrap_or_else(|| {
        path.file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "atlas-project".to_string())
    });

    let project_type = match args.project_type.as_str() {
        "evm" => ProjectType::Evm,
        "svm" => ProjectType::Svm,
        "dual" | _ => ProjectType::Dual,
    };

    println!("{} Initializing Atlas Sphere project...", "→".blue());

    let project = Project::init(path, &name, project_type)?;

    println!("{} Created project: {}", "✓".green(), project.config.name);
    println!("{} Created configuration: x3.toml", "✓".green());
    println!("{} Created directory structure", "✓".green());

    println!();
    println!("Next steps:");
    println!("  1. Add contracts to contracts/");
    println!("  2. Run {} to compile", "x3 build".cyan());
    println!("  3. Run {} to test", "x3 test".cyan());
    println!("  4. Run {} to deploy", "x3 deploy".cyan());

    Ok(())
}
