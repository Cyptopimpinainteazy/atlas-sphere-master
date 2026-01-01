//! Prometheus metrics for the indexer.

use prometheus::{Histogram, HistogramOpts, IntCounter, IntGauge, Registry};
use std::sync::Arc;

/// Metrics collection.
#[derive(Clone)]
pub struct Metrics {
    registry: Arc<Registry>,
    blocks_indexed: IntCounter,
    latest_block: IntGauge,
    block_processing_time: Histogram,
    extrinsics_indexed: IntCounter,
    events_indexed: IntCounter,
    comits_indexed: IntCounter,
    errors: IntCounter,
    db_query_time: Histogram,
}

impl Metrics {
    /// Create a new metrics collection.
    pub fn new() -> Self {
        let registry = Registry::new();

        let blocks_indexed =
            IntCounter::new("indexer_blocks_indexed_total", "Total blocks indexed")
                .expect("metric creation failed");

        let latest_block = IntGauge::new("indexer_latest_block", "Latest indexed block number")
            .expect("metric creation failed");

        let block_processing_time = Histogram::with_opts(HistogramOpts::new(
            "indexer_block_processing_ms",
            "Block processing time in milliseconds",
        ))
        .expect("metric creation failed");

        let extrinsics_indexed = IntCounter::new(
            "indexer_extrinsics_indexed_total",
            "Total extrinsics indexed",
        )
        .expect("metric creation failed");

        let events_indexed =
            IntCounter::new("indexer_events_indexed_total", "Total events indexed")
                .expect("metric creation failed");

        let comits_indexed = IntCounter::new(
            "indexer_comits_indexed_total",
            "Total Comit transactions indexed",
        )
        .expect("metric creation failed");

        let errors = IntCounter::new("indexer_errors_total", "Total indexer errors")
            .expect("metric creation failed");

        let db_query_time = Histogram::with_opts(HistogramOpts::new(
            "indexer_db_query_ms",
            "Database query time in milliseconds",
        ))
        .expect("metric creation failed");

        // Register all metrics
        registry.register(Box::new(blocks_indexed.clone())).unwrap();
        registry.register(Box::new(latest_block.clone())).unwrap();
        registry
            .register(Box::new(block_processing_time.clone()))
            .unwrap();
        registry
            .register(Box::new(extrinsics_indexed.clone()))
            .unwrap();
        registry.register(Box::new(events_indexed.clone())).unwrap();
        registry.register(Box::new(comits_indexed.clone())).unwrap();
        registry.register(Box::new(errors.clone())).unwrap();
        registry.register(Box::new(db_query_time.clone())).unwrap();

        Self {
            registry: Arc::new(registry),
            blocks_indexed,
            latest_block,
            block_processing_time,
            extrinsics_indexed,
            events_indexed,
            comits_indexed,
            errors,
            db_query_time,
        }
    }

    /// Get the prometheus registry.
    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// Record a block being indexed.
    pub fn record_block_indexed(&self, block_number: u64) {
        self.blocks_indexed.inc();
        self.latest_block.set(block_number as i64);
    }

    /// Record block processing time.
    pub fn record_block_time(&self, ms: u64) {
        self.block_processing_time.observe(ms as f64);
    }

    /// Record extrinsics being indexed.
    pub fn record_extrinsics(&self, count: u64) {
        self.extrinsics_indexed.inc_by(count);
    }

    /// Record events being indexed.
    pub fn record_events(&self, count: u64) {
        self.events_indexed.inc_by(count);
    }

    /// Record a Comit transaction.
    pub fn record_comit(&self) {
        self.comits_indexed.inc();
    }

    /// Record an error.
    pub fn record_error(&self) {
        self.errors.inc();
    }

    /// Record database query time.
    pub fn record_db_query(&self, ms: u64) {
        self.db_query_time.observe(ms as f64);
    }

    /// Get total blocks indexed.
    pub fn total_blocks(&self) -> u64 {
        self.blocks_indexed.get()
    }

    /// Get latest block number.
    pub fn latest_block(&self) -> i64 {
        self.latest_block.get()
    }

    /// Get total errors.
    pub fn total_errors(&self) -> u64 {
        self.errors.get()
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}
