// crates/x3-bench/src/main.rs
// X3 Benchmarking harness

use anyhow::Result;
use clap::Parser;
use log::info;
use std::path::PathBuf;

#[cfg(test)]
mod smoke_tests;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    config: PathBuf,
}

fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();
    info!("Loading benchmark config: {}", args.config.display());

    let config_content = std::fs::read_to_string(&args.config)?;
    info!("Config content:\n{}", config_content);

    // Parse TOML config
    #[derive(serde::Deserialize, Debug)]
    struct Benchmark {
        name: String,
        x3_files: Vec<String>,
        iterations: Option<u32>,
    }

    #[derive(serde::Deserialize, Debug)]
    struct BenchConfig {
        benchmarks: Vec<Benchmark>,
    }

    let config: BenchConfig = toml::from_str(&config_content)?;

    for bench in config.benchmarks.iter() {
        let iters = bench.iterations.unwrap_or(1);
        info!("Benchmark '{}' with {} x3 file(s), iterations={}", bench.name, bench.x3_files.len(), iters);
        // Placeholder: compile files and run timing (not implemented here)
    }

    info!("✓ Benchmarks parsed (placeholder run)");
    Ok(())
}
