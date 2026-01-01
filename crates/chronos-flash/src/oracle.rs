//! ChronosFlash Oracle - The main orchestrator
//!
//! Brings together mempool scanning, intent prediction, routing, and pre-execution

use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use crate::config::ChronosConfig;
use crate::error::{ChronosError, ChronosResult};
use crate::intent::SwapIntent;
use crate::mempool::MempoolScanner;
use crate::predictor::IntentPredictor;
use crate::router::QuantumRouter;
use crate::timewarp::{TimeWarpEngine, Signer};
use crate::types::{ExecutionResult, MempoolStats, OracleMetrics, PreSignedBundle, TradeRoute};

/// ChronosFlash Oracle - Negative-latency pre-execution system
///
/// # Architecture
///
/// ```text
/// ┌─────────────────────────────────────────────────────────────┐
/// │                    ChronosFlash Oracle                       │
/// │  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐        │
/// │  │  Mempool    │──▶│   Intent    │──▶│  Quantum    │        │
/// │  │  Scanner    │   │  Predictor  │   │   Router    │        │
/// │  │ (103 chains)│   │  (AI Swarm) │   │  (Evolution)│        │
/// │  └─────────────┘   └─────────────┘   └─────────────┘        │
/// │         │                 │                 │               │
/// │         ▼                 ▼                 ▼               │
/// │  ┌─────────────────────────────────────────────────────┐    │
/// │  │              TimeWarp Pre-Execution Engine          │    │
/// │  │  ┌──────────┐  ┌──────────┐  ┌──────────────────┐   │    │
/// │  │  │Checkpoint│  │Pre-Sign  │  │  Execute Bundle  │   │    │
/// │  │  │ Creation │──│ Bundle   │──│ (-200ms latency) │   │    │
/// │  │  └──────────┘  └──────────┘  └──────────────────┘   │    │
/// │  └─────────────────────────────────────────────────────┘    │
/// └─────────────────────────────────────────────────────────────┘
/// ```
pub struct ChronosOracle {
    config: ChronosConfig,
    mempool: MempoolScanner,
    predictor: IntentPredictor,
    router: QuantumRouter,
    timewarp: TimeWarpEngine,
    signer: Arc<dyn Signer + Send + Sync>,
    metrics: Arc<RwLock<OracleMetrics>>,
    intent_rx: mpsc::Receiver<SwapIntent>,
    running: Arc<RwLock<bool>>,
}

impl ChronosOracle {
    /// Create new ChronosFlash oracle
    pub async fn new(
        config: ChronosConfig,
        signer: Arc<dyn Signer + Send + Sync>,
    ) -> ChronosResult<Self> {
        let (intent_tx, intent_rx) = mpsc::channel(10000);

        Ok(Self {
            mempool: MempoolScanner::new(config.clone(), intent_tx),
            predictor: IntentPredictor::new(config.predictor.clone()),
            router: QuantumRouter::new(config.router.clone()),
            timewarp: TimeWarpEngine::new(),
            signer,
            config,
            metrics: Arc::new(RwLock::new(OracleMetrics::default())),
            intent_rx,
            running: Arc::new(RwLock::new(false)),
        })
    }

    /// Start the oracle
    pub async fn start(&mut self) -> ChronosResult<()> {
        {
            let mut running = self.running.write().await;
            if *running {
                return Err(ChronosError::AlreadyRunning);
            }
            *running = true;
        }

        log::info!("🚀 ChronosFlash Oracle starting...");
        log::info!("   Monitoring {} chains", self.config.chains.len());
        log::info!("   Target latency: {}ms", self.config.target_latency_ms);
        log::info!("   Max time-warp: {}ms", self.config.max_timewarp_ms);

        // Start mempool scanner in background
        let mut mempool = std::mem::replace(
            &mut self.mempool,
            MempoolScanner::new(self.config.clone(), mpsc::channel(1).0),
        );
        tokio::spawn(async move {
            if let Err(e) = mempool.start().await {
                log::error!("Mempool scanner error: {}", e);
            }
        });

        // Main processing loop
        self.run_processing_loop().await
    }

    /// Main processing loop
    async fn run_processing_loop(&mut self) -> ChronosResult<()> {
        log::info!("⚡ ChronosFlash processing loop started");

        loop {
            // Check if still running
            if !*self.running.read().await {
                break;
            }

            // Receive intents from mempool scanner
            match self.intent_rx.recv().await {
                Some(intent) => {
                    // Process intent
                    if let Err(e) = self.process_intent(intent).await {
                        log::warn!("Intent processing failed: {}", e);
                    }
                }
                None => {
                    // Channel closed
                    log::warn!("Intent channel closed");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Process detected swap intent
    async fn process_intent(&mut self, intent: SwapIntent) -> ChronosResult<()> {
        let start = std::time::Instant::now();
        log::debug!(
            "Processing intent: {} -> {} ({})",
            intent.token_in.symbol,
            intent.token_out.symbol,
            intent.amount_in
        );

        // Update predictor with observation
        self.predictor.observe(&intent).await;

        // Check if we should try to front-run this intent
        if !self.should_process(&intent) {
            return Ok(());
        }

        // Step 1: Compute optimal routes
        let routes = self.router.compute_routes(&intent).await?;
        if routes.is_empty() {
            return Err(ChronosError::RouteFailed("No routes found".to_string()));
        }

        let best_route = routes.into_iter().next().unwrap();
        log::debug!("Best route: {} hops, expected output: {}", best_route.hops.len(), best_route.expected_output);

        // Step 2: Check if route is profitable
        if !self.is_profitable(&best_route, &intent) {
            log::debug!("Route not profitable, skipping");
            return Ok(());
        }

        // Step 3: Pre-sign atomic bundle
        let bundle = self.timewarp.presign_bundle(best_route, self.signer.as_ref()).await?;
        
        // Step 4: Execute bundle (TIME-WARP!)
        let result = self.timewarp.execute_bundle(&bundle).await?;
        
        // Update metrics
        self.update_metrics(&result).await;

        let elapsed = start.elapsed();
        log::info!(
            "✨ Time-warp complete! Latency: {}ms, Time advantage: {}ms",
            elapsed.as_millis(),
            result.time_advantage_ms.abs()
        );

        Ok(())
    }

    /// Check if intent should be processed
    fn should_process(&self, intent: &SwapIntent) -> bool {
        // Check confidence threshold
        if intent.confidence < self.config.prediction_threshold {
            return false;
        }

        // Check minimum value threshold
        let min_value = 1_000_000_000_000_000_000u128; // 1 token
        if intent.amount_in < min_value {
            return false;
        }

        true
    }

    /// Check if route is profitable after gas
    fn is_profitable(&self, route: &TradeRoute, intent: &SwapIntent) -> bool {
        // Simple check: output > input + estimated gas cost
        let gas_cost_estimate = route.total_gas as u128 * 50_000_000_000u128; // 50 gwei
        let profit = route.expected_output.saturating_sub(intent.amount_in);
        
        profit > gas_cost_estimate
    }

    /// Update oracle metrics
    async fn update_metrics(&self, result: &ExecutionResult) {
        let mut metrics = self.metrics.write().await;
        
        if result.success {
            metrics.bundles_executed += 1;
            metrics.total_volume += result.actual_output;
            metrics.total_gas_saved += result.gas_used;
            
            // Update averages
            let n = metrics.bundles_executed as f64;
            metrics.avg_latency_ms = (metrics.avg_latency_ms * (n - 1.0) + result.latency_ms as f64) / n;
            metrics.avg_time_advantage_ms = (metrics.avg_time_advantage_ms * (n - 1.0) + result.time_advantage_ms.abs() as f64) / n;
            
            metrics.success_rate = metrics.bundles_executed as f64 / (metrics.bundles_executed + 1) as f64;
        }
    }

    /// Get oracle metrics
    pub async fn get_metrics(&self) -> OracleMetrics {
        self.metrics.read().await.clone()
    }

    /// Stop the oracle
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
        log::info!("ChronosFlash Oracle stopped");
    }

    /// Process predicted intents (before they appear in mempool)
    pub async fn process_predictions(&mut self) -> ChronosResult<Vec<PreSignedBundle>> {
        let mut bundles = vec![];

        // Get chains to monitor
        for (chain_id, _) in &self.config.chains {
            // Get predictions for this chain
            let predictions = self.predictor.predict(*chain_id).await?;
            
            for prediction in predictions {
                if prediction.confidence >= self.config.prediction_threshold {
                    // Convert prediction to synthetic intent
                    let synthetic_intent = SwapIntent {
                        id: prediction.id,
                        chain_id: prediction.chain_id,
                        sender: prediction.predicted_sender,
                        token_in: prediction.token_in,
                        token_out: prediction.token_out,
                        amount_in: (prediction.predicted_amount_range.0 + prediction.predicted_amount_range.1) / 2,
                        min_amount_out: 0,
                        deadline: 0,
                        tx_hash: [0u8; 32],
                        gas_price: 50_000_000_000, // 50 gwei default
                        gas_limit: 500_000,
                        detected_at: chrono::Utc::now().timestamp_millis() as u64,
                        confidence: prediction.confidence,
                        intent_type: crate::intent::IntentType::SimpleSwap,
                        metadata: crate::intent::IntentMetadata::default(),
                    };

                    // Compute routes
                    if let Ok(routes) = self.router.compute_routes(&synthetic_intent).await {
                        if let Some(route) = routes.into_iter().next() {
                            // Pre-sign bundle
                            if let Ok(bundle) = self.timewarp.presign_bundle(route, self.signer.as_ref()).await {
                                bundles.push(bundle);
                            }
                        }
                    }
                }
            }
        }

        log::info!("Pre-signed {} bundles from predictions", bundles.len());
        Ok(bundles)
    }
}

/// Builder for ChronosOracle
pub struct ChronosOracleBuilder {
    config: ChronosConfig,
    signer: Option<Arc<dyn Signer + Send + Sync>>,
}

impl ChronosOracleBuilder {
    pub fn new() -> Self {
        Self {
            config: ChronosConfig::default(),
            signer: None,
        }
    }

    pub fn with_config(mut self, config: ChronosConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_signer(mut self, signer: Arc<dyn Signer + Send + Sync>) -> Self {
        self.signer = Some(signer);
        self
    }

    pub async fn build(self) -> ChronosResult<ChronosOracle> {
        let signer = self.signer
            .ok_or_else(|| ChronosError::InvalidConfig("Signer required".to_string()))?;

        ChronosOracle::new(self.config, signer).await
    }
}

impl Default for ChronosOracleBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockSigner;

    #[async_trait::async_trait]
    impl Signer for MockSigner {
        async fn sign(&self, _chain_id: crate::types::ChainId, _data: &[u8]) -> ChronosResult<crate::types::Signature> {
            Ok(crate::types::Signature {
                chain_id: 1,
                signer: [0u8; 32],
                signature: vec![0u8; 65],
                recovery_id: 0,
            })
        }

        fn address(&self) -> crate::types::Address {
            [0u8; 32]
        }
    }

    #[tokio::test]
    async fn test_oracle_creation() {
        let config = ChronosConfig::default();
        let signer = Arc::new(MockSigner);
        
        let oracle = ChronosOracle::new(config, signer).await;
        assert!(oracle.is_ok());
    }

    #[tokio::test]
    async fn test_oracle_builder() {
        let oracle = ChronosOracleBuilder::new()
            .with_config(ChronosConfig::default())
            .with_signer(Arc::new(MockSigner))
            .build()
            .await;

        assert!(oracle.is_ok());
    }
}
