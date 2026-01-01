//! Apotheosis Transaction Executor

use crate::{
    error::{ApotheosisError, ApotheosisResult},
    types::*,
};
use chrono::Utc;
use tokio::sync::mpsc;
use tracing::{info, warn, error};

/// Executor for Apotheosis transactions
pub struct ApotheosisExecutor {
    /// Event sender
    event_tx: Option<mpsc::Sender<ApotheosisEvent>>,
    /// Maximum retries per chain
    max_retries: u32,
    /// Retry delay (ms)
    retry_delay_ms: u64,
}

/// Event types during execution
#[derive(Debug, Clone)]
pub enum ApotheosisEvent {
    /// Execution started
    Started { tx_id: String },
    /// Chain execution started
    ChainStarted { chain_id: ChainId },
    /// Asset migration started
    AssetStarted { chain_id: ChainId, asset_index: usize },
    /// Asset migration completed
    AssetCompleted { chain_id: ChainId, asset_index: usize, tx_hash: String },
    /// Chain execution completed
    ChainCompleted { chain_id: ChainId, tx_hashes: Vec<String> },
    /// Chain execution failed
    ChainFailed { chain_id: ChainId, error: String },
    /// All chains completed
    Completed { tx_id: String, stats: ApotheosisStats },
    /// Execution failed
    Failed { tx_id: String, error: String },
    /// Rollback initiated
    RollbackInitiated { tx_id: String },
    /// Rollback completed
    RollbackCompleted { tx_id: String },
}

/// Result of executing a route
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Transaction hash
    pub tx_hash: String,
    /// Gas used
    pub gas_used: u128,
    /// Success
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

impl ApotheosisExecutor {
    /// Create a new executor
    pub fn new() -> Self {
        Self {
            event_tx: None,
            max_retries: 3,
            retry_delay_ms: 5000,
        }
    }

    /// Set event channel for status updates
    pub fn with_events(mut self, tx: mpsc::Sender<ApotheosisEvent>) -> Self {
        self.event_tx = Some(tx);
        self
    }

    /// Set max retries
    pub fn max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// Execute an Apotheosis transaction (simulated)
    pub async fn execute(&self, mut tx: ApotheosisTransaction) -> ApotheosisResult<ApotheosisTransaction> {
        info!("Starting Apotheosis execution: {}", tx.id);
        self.emit_event(ApotheosisEvent::Started { tx_id: tx.id.clone() }).await;

        tx.status = TransactionStatus::Executing;
        tx.started_at = Some(Utc::now());

        let mut completed_chains: Vec<ChainId> = Vec::new();
        let mut failed_chains: Vec<(ChainId, String)> = Vec::new();

        // Execute routes grouped by source chain
        for source in &tx.sources {
            let chain_id = source.chain_id;
            
            self.emit_event(ApotheosisEvent::ChainStarted { chain_id }).await;

            // Get routes for this chain
            let chain_routes: Vec<&MigrationRoute> = tx.routes.iter()
                .filter(|r| r.asset.source_chain == chain_id)
                .collect();

            let mut chain_tx_hashes = Vec::new();
            let mut chain_failed = false;

            for (idx, route) in chain_routes.iter().enumerate() {
                self.emit_event(ApotheosisEvent::AssetStarted { 
                    chain_id, 
                    asset_index: idx 
                }).await;

                match self.execute_route_simulated(route, chain_id).await {
                    Ok(result) => {
                        info!("Asset {} migrated on chain {}: {}", idx, chain_id.0, result.tx_hash);
                        chain_tx_hashes.push(result.tx_hash.clone());
                        self.emit_event(ApotheosisEvent::AssetCompleted { 
                            chain_id, 
                            asset_index: idx,
                            tx_hash: result.tx_hash,
                        }).await;
                    }
                    Err(e) => {
                        error!("Failed to migrate asset {} on chain {}: {}", idx, chain_id.0, e);
                        chain_failed = true;
                        failed_chains.push((chain_id, e.to_string()));
                        break;
                    }
                }
            }

            if chain_failed {
                self.emit_event(ApotheosisEvent::ChainFailed { 
                    chain_id, 
                    error: "Asset migration failed".to_string() 
                }).await;

                tx.chain_statuses.push(ChainExecutionStatus {
                    chain_id,
                    status: TransactionStatus::Failed,
                    tx_hashes: chain_tx_hashes,
                    error: Some("Migration failed".to_string()),
                    block_number: None,
                    timestamp: Utc::now(),
                });
            } else {
                completed_chains.push(chain_id);
                self.emit_event(ApotheosisEvent::ChainCompleted { 
                    chain_id, 
                    tx_hashes: chain_tx_hashes.clone() 
                }).await;

                tx.chain_statuses.push(ChainExecutionStatus {
                    chain_id,
                    status: TransactionStatus::Completed,
                    tx_hashes: chain_tx_hashes,
                    error: None,
                    block_number: None,
                    timestamp: Utc::now(),
                });
            }
        }

        // Determine final status
        let total_chains = tx.sources.len();
        let completed_count = completed_chains.len();

        if failed_chains.is_empty() {
            tx.status = TransactionStatus::Completed;
            tx.completed_at = Some(Utc::now());

            let stats = self.calculate_stats(&tx);
            self.emit_event(ApotheosisEvent::Completed { 
                tx_id: tx.id.clone(), 
                stats 
            }).await;

            info!("Apotheosis completed successfully: {}", tx.id);
        } else if completed_count > 0 {
            tx.status = TransactionStatus::PartiallyCompleted;
            tx.completed_at = Some(Utc::now());

            warn!("Apotheosis partially completed: {}/{} chains", completed_count, total_chains);
        } else {
            tx.status = TransactionStatus::Failed;
            
            let error_msg = failed_chains.iter()
                .map(|(c, e)| format!("Chain {}: {}", c.0, e))
                .collect::<Vec<_>>()
                .join("; ");

            self.emit_event(ApotheosisEvent::Failed { 
                tx_id: tx.id.clone(), 
                error: error_msg 
            }).await;

            error!("Apotheosis failed: {}", tx.id);
        }

        Ok(tx)
    }

    /// Execute a single route (simulated)
    async fn execute_route_simulated(
        &self,
        route: &MigrationRoute,
        _chain_id: ChainId,
    ) -> ApotheosisResult<ExecutionResult> {
        // Simulate execution delay
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Generate mock transaction hash
        let tx_hash = format!("0x{:064x}", rand::random::<u64>());

        Ok(ExecutionResult {
            tx_hash,
            gas_used: route.hops.iter().map(|h| h.gas_cost).sum(),
            success: true,
            error: None,
        })
    }

    /// Calculate final stats
    fn calculate_stats(&self, tx: &ApotheosisTransaction) -> ApotheosisStats {
        let time_taken = tx.started_at
            .and_then(|start| tx.completed_at.map(|end| (end - start).num_seconds() as u64))
            .unwrap_or(0);

        let successful_chains = tx.chain_statuses.iter()
            .filter(|s| s.status == TransactionStatus::Completed)
            .count();

        ApotheosisStats {
            chains_migrated: successful_chains,
            assets_migrated: tx.total_assets(),
            total_value_usd: tx.total_value_usd(),
            total_gas_usd: tx.total_cost_usd,
            time_taken,
            success_rate: if tx.sources.is_empty() { 0.0 } else { successful_chains as f64 / tx.sources.len() as f64 },
        }
    }

    /// Emit an event if event channel is configured
    async fn emit_event(&self, event: ApotheosisEvent) {
        if let Some(tx) = &self.event_tx {
            let _ = tx.send(event).await;
        }
    }
}

impl Default for ApotheosisExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_executor_creation() {
        let executor = ApotheosisExecutor::new();
        assert_eq!(executor.max_retries, 3);
    }
}
