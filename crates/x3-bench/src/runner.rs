//! Benchmark runner implementation.

use anyhow::{Context, Result};
use prettytable::{row, Table};
use serde::Serialize;
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};

use crate::pipeline::{compile_optimized, compile_unoptimized};
use x3_opt::telemetry::{mine_frequent_ngrams, RunTelemetry};

// ---------- Config ----------

pub struct BenchConfig {
    pub max_opt_iters: usize,
    pub output_dir: PathBuf,
    pub csv_filename: String,
}

#[derive(Serialize, Debug)]
pub struct BenchResult {
    pub name: String,

    // Metrics
    pub old_instrs: usize,
    pub new_instrs: usize,
    pub instr_delta: isize,

    pub old_gas: u64,
    pub new_gas: u64,
    pub gas_delta: i64,

    pub old_bytes: usize,
    pub new_bytes: usize,
    pub bytes_delta: isize,

    pub notes: String,

    // Raw artifacts for debugging (not serialized to CSV)
    #[serde(skip_serializing)]
    pub old_bytecode: Vec<u8>,
    #[serde(skip_serializing)]
    pub new_bytecode: Vec<u8>,
    #[serde(skip_serializing)]
    pub source: String,
}

// ---------- Public API ----------

/// Run benchmarks on a sample suite.
/// `samples` is a Vec of (name, source_str).
pub fn run_benchmarks_and_report(
    cfg: &BenchConfig,
    samples: &[(&'static str, &'static str)],
    outdir: &Path,
) -> Result<Vec<BenchResult>> {
    create_dir_all(outdir).context("create output dir")?;
    let mut telemetry = RunTelemetry::new();

    let mut table = Table::new();
    table.add_row(row![
        "Sample",
        "Old Instrs",
        "New Instrs",
        "Δ Instrs",
        "Old Gas",
        "New Gas",
        "Δ Gas",
        "Old Bytes",
        "New Bytes",
        "Δ Bytes"
    ]);

    let mut results = Vec::new();

    for (name, src) in samples.iter() {
        println!("┌─────────────────────────────────────────────────────────────");
        println!("│ Running benchmark: {}", name);
        println!("└─────────────────────────────────────────────────────────────");

        // OLD pipeline (no optimizer)
        let old_result = match compile_unoptimized(src) {
            Ok(r) => r,
            Err(e) => {
                println!("  ⚠️  SKIP (compile error): {}", e);
                continue;
            }
        };

        let old_instrs = old_result.stats.instruction_count;
        let old_gas = old_result.stats.gas_estimate;
        let old_bytes = old_result.stats.bytecode_size;
        let old_bytecode = old_result.bytecode_bytes;

        println!(
            "  📊 Baseline: {} instrs, {} gas, {} bytes",
            old_instrs, old_gas, old_bytes
        );

        // NEW pipeline (with optimizer)
        let new_result =
            match compile_optimized(src, cfg.max_opt_iters, Some(&mut telemetry), Some(*name)) {
                Ok(r) => r,
                Err(e) => {
                    println!("  ⚠️  Optimizer error (using baseline): {}", e);
                    // Use old results as fallback
                    results.push(BenchResult {
                        name: (*name).to_string(),
                        old_instrs,
                        new_instrs: old_instrs,
                        instr_delta: 0,
                        old_gas,
                        new_gas: old_gas,
                        gas_delta: 0,
                        old_bytes,
                        new_bytes: old_bytes,
                        bytes_delta: 0,
                        notes: format!("optimizer error: {}", e),
                        old_bytecode: old_bytecode.clone(),
                        new_bytecode: old_bytecode,
                        source: src.to_string(),
                    });
                    continue;
                }
            };

        let new_instrs = new_result.stats.instruction_count;
        let new_gas = new_result.stats.gas_estimate;
        let new_bytes = new_result.stats.bytecode_size;
        let new_bytecode = new_result.bytecode_bytes;

        println!(
            "  📊 Optimized: {} instrs, {} gas, {} bytes",
            new_instrs, new_gas, new_bytes
        );

        // Compute deltas
        let instr_delta = new_instrs as isize - old_instrs as isize;
        let gas_delta = new_gas as i64 - old_gas as i64;
        let bytes_delta = new_bytes as isize - old_bytes as isize;

        // Format delta indicators
        let instr_indicator = delta_indicator(instr_delta);
        let gas_indicator = delta_indicator(gas_delta as isize);
        let bytes_indicator = delta_indicator(bytes_delta);

        println!(
            "  📈 Delta: {} instrs {}, {} gas {}, {} bytes {}",
            instr_delta, instr_indicator, gas_delta, gas_indicator, bytes_delta, bytes_indicator
        );

        let notes = format!(
            "instrΔ={}{} gasΔ={}{} bytesΔ={}{}",
            instr_delta, instr_indicator, gas_delta, gas_indicator, bytes_delta, bytes_indicator
        );

        // Add to table
        table.add_row(row![
            name,
            old_instrs,
            new_instrs,
            format!("{}{}", instr_delta, instr_indicator),
            old_gas,
            new_gas,
            format!("{}{}", gas_delta, gas_indicator),
            old_bytes,
            new_bytes,
            format!("{}{}", bytes_delta, bytes_indicator)
        ]);

        results.push(BenchResult {
            name: (*name).to_string(),
            old_instrs,
            new_instrs,
            instr_delta,
            old_gas,
            new_gas,
            gas_delta,
            old_bytes,
            new_bytes,
            bytes_delta,
            notes,
            old_bytecode,
            new_bytecode,
            source: src.to_string(),
        });
    }

    println!();
    println!("═══════════════════════════════════════════════════════════════");
    println!("                     Benchmark Summary");
    println!("═══════════════════════════════════════════════════════════════");
    println!();
    table.printstd();

    // Print aggregate stats
    if !results.is_empty() {
        let total_old_gas: u64 = results.iter().map(|r| r.old_gas).sum();
        let total_new_gas: u64 = results.iter().map(|r| r.new_gas).sum();
        let total_old_bytes: usize = results.iter().map(|r| r.old_bytes).sum();
        let total_new_bytes: usize = results.iter().map(|r| r.new_bytes).sum();

        println!();
        println!("Aggregate Stats ({} samples):", results.len());
        println!(
            "  Total Gas:   {} → {} (Δ {}{})",
            total_old_gas,
            total_new_gas,
            total_new_gas as i64 - total_old_gas as i64,
            delta_indicator((total_new_gas as i64 - total_old_gas as i64) as isize)
        );
        println!(
            "  Total Bytes: {} → {} (Δ {}{})",
            total_old_bytes,
            total_new_bytes,
            total_new_bytes as isize - total_old_bytes as isize,
            delta_indicator(total_new_bytes as isize - total_old_bytes as isize)
        );

        if total_old_gas > 0 {
            let gas_reduction_pct = 100.0 * (1.0 - (total_new_gas as f64 / total_old_gas as f64));
            if gas_reduction_pct > 0.0 {
                println!("  Gas Reduction: {:.1}%", gas_reduction_pct);
            }
        }
    }

    println!();
    match telemetry.write("target/x3-opt-telemetry") {
        Ok(_) => println!(
            "  🔥 Telemetry written to target/x3-opt-telemetry/{}",
            telemetry.run_id
        ),
        Err(e) => println!("  ⚠️ Telemetry write failed: {}", e),
    }

    for (sample, passes) in telemetry.benches.iter() {
        let bigrams = mine_frequent_ngrams(passes, 2, 5);
        let trigrams = mine_frequent_ngrams(passes, 3, 5);
        if bigrams.is_empty() && trigrams.is_empty() {
            continue;
        }
        println!("Telemetry suggestions for {}:", sample);
        if !bigrams.is_empty() {
            println!("  bigrams: {:?}", bigrams);
        }
        if !trigrams.is_empty() {
            println!("  trigrams: {:?}", trigrams);
        }
    }

    Ok(results)
}

/// Return an indicator emoji based on delta direction.
fn delta_indicator(delta: isize) -> &'static str {
    if delta < 0 {
        " ✓" // Improvement (reduction)
    } else if delta > 0 {
        " ⬆" // Regression (increase)
    } else {
        " =" // No change
    }
}
