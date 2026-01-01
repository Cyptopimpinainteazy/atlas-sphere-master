//! Bench comparator: write and analyze optimization results.
//!
//! This module provides structured reporting for optimizer benchmark runs,
//! enabling comparison across optimization passes and rounds.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Global metrics from a single benchmark round.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct GlobalMetrics {
    pub instr: usize,
    pub gas: u64,
    pub bytes: usize,
}

/// Per-sample metrics.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SampleMetrics {
    pub name: String,
    pub instr: usize,
    pub gas: u64,
    pub bytes: usize,
}

/// Complete benchmark report.
#[derive(Serialize, Deserialize, Debug)]
pub struct Report {
    pub global: GlobalMetrics,
    pub per_sample: Vec<SampleMetrics>,
    pub timestamp: String,
}

impl Report {
    /// Create a new report from per-sample results.
    pub fn new(per_sample: Vec<SampleMetrics>) -> Self {
        let total_instr = per_sample.iter().map(|m| m.instr).sum();
        let total_gas = per_sample.iter().map(|m| m.gas).sum();
        let total_bytes = per_sample.iter().map(|m| m.bytes).sum();

        let timestamp = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();

        Report {
            global: GlobalMetrics {
                instr: total_instr,
                gas: total_gas,
                bytes: total_bytes,
            },
            per_sample,
            timestamp,
        }
    }
}

/// Write a report to JSON file.
pub fn write_report<P: AsRef<Path>>(path: P, report: &Report) -> Result<()> {
    let json = serde_json::to_string_pretty(report)?;
    fs::write(path, json)?;
    Ok(())
}

/// Read a report from JSON file.
pub fn read_report<P: AsRef<Path>>(path: P) -> Result<Report> {
    let json = fs::read_to_string(path)?;
    let report = serde_json::from_str(&json)?;
    Ok(report)
}

/// Compare two reports and return deltas.
pub fn compare_reports(before: &Report, after: &Report) -> ComparisonResult {
    ComparisonResult {
        instr_delta: (after.global.instr as isize) - (before.global.instr as isize),
        gas_delta: (after.global.gas as i64) - (before.global.gas as i64),
        bytes_delta: (after.global.bytes as isize) - (before.global.bytes as isize),
        instr_pct: if before.global.instr > 0 {
            ((after.global.instr as f64 - before.global.instr as f64) / before.global.instr as f64)
                * 100.0
        } else {
            0.0
        },
        gas_pct: if before.global.gas > 0 {
            ((after.global.gas as f64 - before.global.gas as f64) / before.global.gas as f64)
                * 100.0
        } else {
            0.0
        },
    }
}

/// Comparison deltas between two reports.
#[derive(Debug)]
pub struct ComparisonResult {
    pub instr_delta: isize,
    pub gas_delta: i64,
    pub bytes_delta: isize,
    pub instr_pct: f64,
    pub gas_pct: f64,
}

impl std::fmt::Display for ComparisonResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Instr: {} ({:.1}%) | Gas: {} ({:.1}%) | Bytes: {}",
            self.instr_delta, self.instr_pct, self.gas_delta, self.gas_pct, self.bytes_delta
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_compare_reports() {
        let before = Report::new(vec![SampleMetrics {
            name: "test1".into(),
            instr: 100,
            gas: 250,
            bytes: 400,
        }]);

        let after = Report::new(vec![SampleMetrics {
            name: "test1".into(),
            instr: 80,
            gas: 200,
            bytes: 320,
        }]);

        let cmp = compare_reports(&before, &after);
        assert_eq!(cmp.instr_delta, -20);
        assert_eq!(cmp.gas_delta, -50);
    }
}
