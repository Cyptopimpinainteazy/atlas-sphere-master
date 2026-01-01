//! Documentation generation command.

use crate::error::{CliError, Result};
use crate::project::Project;
use clap::Args;
use colored::Colorize;
use std::process::Command;

#[derive(Args)]
pub struct DocgenArgs {
    /// Output directory
    #[arg(short, long, default_value = "docs")]
    pub output: String,

    /// Output format (markdown, html)
    #[arg(short, long, default_value = "markdown")]
    pub format: String,

    /// Include internal functions
    #[arg(long)]
    pub internal: bool,

    /// Generate for specific contract
    pub contract: Option<String>,
}

pub async fn execute(args: DocgenArgs) -> Result<()> {
    let project = Project::load_current()?;

    println!(
        "{} Generating documentation for: {}",
        "→".blue(),
        project.config.name
    );

    // Create output directory
    let output_dir = project.root.join(&args.output);
    std::fs::create_dir_all(&output_dir)?;

    // Find Solidity files
    let sol_files = project.find_solidity_files()?;

    if sol_files.is_empty() {
        println!("{} No Solidity files found", "!".yellow());
        return Ok(());
    }

    // Try to use forge doc
    let forge_result = try_forge_doc(&project, &args);

    match forge_result {
        Ok(()) => {
            println!("{} Documentation generated with forge", "✓".green());
        }
        Err(_) => {
            // Fall back to basic documentation
            println!(
                "{} forge doc not available, generating basic docs...",
                "!".yellow()
            );
            generate_basic_docs(&project, &sol_files, &output_dir, &args)?;
        }
    }

    println!(
        "{} Documentation written to: {}",
        "✓".green(),
        output_dir.display()
    );

    Ok(())
}

fn try_forge_doc(project: &Project, args: &DocgenArgs) -> Result<()> {
    let mut cmd = Command::new("forge");
    cmd.arg("doc");
    cmd.arg("--out").arg(&args.output);

    if args.format == "html" {
        cmd.arg("--build");
    }

    cmd.current_dir(&project.root);

    let output = cmd.output()?;

    if !output.status.success() {
        return Err(CliError::Command("forge doc failed".to_string()));
    }

    Ok(())
}

fn generate_basic_docs(
    project: &Project,
    files: &[std::path::PathBuf],
    output_dir: &std::path::Path,
    args: &DocgenArgs,
) -> Result<()> {
    // Generate index
    let mut index = String::new();
    index.push_str(&format!("# {} Documentation\n\n", project.config.name));
    index.push_str("## Contracts\n\n");

    for file in files {
        let relative = file.strip_prefix(&project.root).unwrap_or(file);
        let name = file
            .file_stem()
            .map(|s| s.to_string_lossy())
            .unwrap_or_default();

        // Skip if filtering by contract
        if let Some(ref filter) = args.contract {
            if !name.contains(filter) {
                continue;
            }
        }

        index.push_str(&format!("- [{}](contracts/{}.md)\n", name, name));

        // Generate contract doc
        let doc = generate_contract_doc(file, args)?;
        let doc_path = output_dir.join("contracts").join(format!("{}.md", name));
        std::fs::create_dir_all(doc_path.parent().unwrap())?;
        std::fs::write(doc_path, doc)?;

        println!("  {} Generated: {}", "✓".green(), relative.display());
    }

    // Write index
    std::fs::write(output_dir.join("README.md"), index)?;

    Ok(())
}

fn generate_contract_doc(file: &std::path::Path, args: &DocgenArgs) -> Result<String> {
    let content = std::fs::read_to_string(file)?;
    let name = file
        .file_stem()
        .map(|s| s.to_string_lossy())
        .unwrap_or_default();

    let mut doc = String::new();
    doc.push_str(&format!("# {}\n\n", name));

    // Extract SPDX license
    if let Some(line) = content
        .lines()
        .find(|l| l.contains("SPDX-License-Identifier"))
    {
        if let Some(license) = line.split(':').nth(1) {
            doc.push_str(&format!("**License:** {}\n\n", license.trim()));
        }
    }

    // Extract pragma
    if let Some(line) = content.lines().find(|l| l.starts_with("pragma")) {
        doc.push_str(&format!("**Solidity:** `{}`\n\n", line.trim()));
    }

    // Extract NatSpec comments
    let mut in_natspec = false;
    let mut current_natspec = String::new();
    let mut functions = Vec::new();
    let mut events = Vec::new();
    let mut errors = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("/**") || trimmed.starts_with("///") {
            in_natspec = true;
            current_natspec.clear();
        }

        if in_natspec {
            let comment = trimmed
                .trim_start_matches("/**")
                .trim_start_matches("///")
                .trim_start_matches("*")
                .trim_start_matches("*/")
                .trim();
            if !comment.is_empty() {
                current_natspec.push_str(comment);
                current_natspec.push(' ');
            }
        }

        if trimmed.contains("*/") || (in_natspec && !trimmed.starts_with("///")) {
            in_natspec = false;
        }

        // Capture function signatures
        if trimmed.starts_with("function ") {
            let visibility = if trimmed.contains("public") {
                "public"
            } else if trimmed.contains("external") {
                "external"
            } else if trimmed.contains("internal") {
                "internal"
            } else if trimmed.contains("private") {
                "private"
            } else {
                "public"
            };

            if visibility == "internal" || visibility == "private" {
                if !args.internal {
                    current_natspec.clear();
                    continue;
                }
            }

            functions.push((trimmed.to_string(), current_natspec.clone()));
            current_natspec.clear();
        }

        // Capture events
        if trimmed.starts_with("event ") {
            events.push((trimmed.to_string(), current_natspec.clone()));
            current_natspec.clear();
        }

        // Capture errors
        if trimmed.starts_with("error ") {
            errors.push((trimmed.to_string(), current_natspec.clone()));
            current_natspec.clear();
        }
    }

    // Write functions
    if !functions.is_empty() {
        doc.push_str("## Functions\n\n");
        for (sig, desc) in functions {
            let func_name = sig.split('(').next().unwrap_or(&sig);
            let func_name = func_name.trim_start_matches("function ");
            doc.push_str(&format!("### `{}`\n\n", func_name));
            doc.push_str(&format!("```solidity\n{}\n```\n\n", sig));
            if !desc.trim().is_empty() {
                doc.push_str(&format!("{}\n\n", desc.trim()));
            }
        }
    }

    // Write events
    if !events.is_empty() {
        doc.push_str("## Events\n\n");
        for (sig, desc) in events {
            doc.push_str(&format!("```solidity\n{}\n```\n\n", sig));
            if !desc.trim().is_empty() {
                doc.push_str(&format!("{}\n\n", desc.trim()));
            }
        }
    }

    // Write errors
    if !errors.is_empty() {
        doc.push_str("## Errors\n\n");
        for (sig, desc) in errors {
            doc.push_str(&format!("```solidity\n{}\n```\n\n", sig));
            if !desc.trim().is_empty() {
                doc.push_str(&format!("{}\n\n", desc.trim()));
            }
        }
    }

    Ok(doc)
}
