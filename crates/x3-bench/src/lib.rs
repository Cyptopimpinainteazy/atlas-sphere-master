//! X3 Benchmark Suite
//! Benchmark infrastructure for measuring optimizer effectiveness.

pub mod comparator;
pub mod pipeline;
pub mod runner;
pub mod samples;

pub use comparator::{
    compare_reports, read_report, write_report, GlobalMetrics, Report, SampleMetrics,
};
