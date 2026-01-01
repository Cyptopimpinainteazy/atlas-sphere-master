//! X3 Sidecar Daemon
//!
//! Off-chain swarm execution node for Atlas Sphere. This daemon:
//! - Connects to the swarm network
//! - Receives X3 bytecode execution jobs
//! - Executes jobs in a sandboxed VM
//! - Generates deterministic receipts with Merkle proofs
//! - Submits receipts to the on-chain verifier
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                        X3 SIDECAR DAEMON                            │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │                                                                     │
//! │  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐            │
//! │  │    RPC       │   │    Job       │   │   Receipt    │            │
//! │  │   Server     │──▶│   Queue      │──▶│  Generator   │            │
//! │  └──────────────┘   └──────────────┘   └──────────────┘            │
//! │         │                  │                  │                    │
//! │         │                  ▼                  │                    │
//! │         │         ┌──────────────┐           │                    │
//! │         │         │    X3 VM     │           │                    │
//! │         │         │  Executor    │           │                    │
//! │         │         └──────────────┘           │                    │
//! │         │                  │                  │                    │
//! │         ▼                  ▼                  ▼                    │
//! │  ┌──────────────────────────────────────────────────┐              │
//! │  │              State Manager                        │              │
//! │  │  • Merkle Tree  • Checkpoints  • Rollback        │              │
//! │  └──────────────────────────────────────────────────┘              │
//! │                           │                                        │
//! │                           ▼                                        │
//! │  ┌──────────────────────────────────────────────────┐              │
//! │  │              Chain Submitter                      │              │
//! │  │  • Receipt Submission  • Gas Estimation          │              │
//! │  └──────────────────────────────────────────────────┘              │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```

pub mod config;
pub mod executor;
pub mod job;
pub mod receipt;
pub mod rpc;
pub mod state;
pub mod submitter;
pub mod telemetry;

use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

pub use config::SidecarConfig;
pub use executor::X3Executor;
pub use job::{Job, JobQueue};
pub use receipt::{ExecutionReceipt, ReceiptGenerator};
pub use state::StateManager;
pub use submitter::ChainSubmitter;
pub use telemetry::Telemetry;

/// Sidecar state (shared across components)
pub struct SidecarState {
    pub start_time: Instant,
    pub jobs_completed: u64,
    pub jobs_failed: u64,
    pub registered: bool,
}

impl Default for SidecarState {
    fn default() -> Self {
        Self {
            start_time: Instant::now(),
            jobs_completed: 0,
            jobs_failed: 0,
            registered: false,
        }
    }
}

/// Sidecar daemon
pub struct SidecarDaemon {
    pub config: SidecarConfig,
    pub job_queue: Arc<RwLock<JobQueue>>,
    pub executor: Arc<X3Executor>,
    pub state_manager: Arc<RwLock<StateManager>>,
    pub receipt_generator: Arc<ReceiptGenerator>,
    pub submitter: Arc<ChainSubmitter>,
    pub telemetry: Arc<Telemetry>,
    pub state: Arc<RwLock<SidecarState>>,
}

impl SidecarDaemon {
    /// Create a new sidecar daemon
    pub fn new(config: SidecarConfig) -> anyhow::Result<Self> {
        let state_manager = Arc::new(RwLock::new(StateManager::new()));
        let executor = Arc::new(X3Executor::new(config.vm.clone()));
        let receipt_generator = Arc::new(ReceiptGenerator::from_hex(&config.executor_key)?);
        let submitter = Arc::new(ChainSubmitter::new(
            config.chain_rpc.clone(),
            config.executor_key.clone(),
        ));
        let job_queue = Arc::new(RwLock::new(JobQueue::new()));
        let telemetry = Telemetry::new();
        let state = Arc::new(RwLock::new(SidecarState::default()));

        Ok(Self {
            config,
            job_queue,
            executor,
            state_manager,
            receipt_generator,
            submitter,
            telemetry,
            state,
        })
    }

    /// Run the daemon
    pub async fn run(self: Arc<Self>) -> anyhow::Result<()> {
        info!("Starting X3 Sidecar Daemon v{}", env!("CARGO_PKG_VERSION"));
        info!("RPC server on port {}", self.config.rpc_port);
        info!("Metrics on port {}", self.config.metrics_port);

        // Build RPC state
        let rpc_state = Arc::new(rpc::RpcState {
            job_queue: Arc::clone(&self.job_queue),
            sidecar_state: Arc::clone(&self.state),
        });

        // Start RPC server
        let rpc_addr = format!("0.0.0.0:{}", self.config.rpc_port);
        let router = rpc::create_router(rpc_state);

        info!("RPC server listening on {}", rpc_addr);

        // Spawn job processor
        let daemon = Arc::clone(&self);
        let processor_handle = tokio::spawn(async move {
            daemon.job_processor_loop().await;
        });

        // Run RPC server (axum 0.6 style)
        axum::Server::bind(&rpc_addr.parse()?)
            .serve(router.into_make_service())
            .await?;

        processor_handle.abort();
        Ok(())
    }

    /// Job processor loop
    async fn job_processor_loop(&self) {
        loop {
            // Try to get next job
            let job = {
                let mut queue = self.job_queue.write().await;
                queue.pop()
            };

            if let Some(job) = job {
                let timer = telemetry::ExecutionTimer::start(Arc::clone(&self.telemetry));

                // Create checkpoint
                {
                    let mut sm = self.state_manager.write().await;
                    sm.checkpoint();
                }

                // Execute
                match self
                    .executor
                    .execute(&job.bytecode, &job.input, job.gas_limit)
                {
                    Ok(result) => {
                        timer.complete(result.gas_used);

                        // Get pre and post state managers
                        let pre_state = StateManager::new();
                        let post_state = self.state_manager.read().await;

                        // Generate receipt
                        let receipt = self.receipt_generator.generate(
                            job.id,
                            &job.input,
                            &result,
                            &pre_state,
                            &*post_state,
                        );

                        // Submit to chain
                        match self.submitter.submit_receipt(&receipt).await {
                            Ok(tx_hash) => {
                                info!("Receipt submitted: {}", tx_hash);
                                self.telemetry.record_receipt_submitted();
                                let mut state = self.state.write().await;
                                state.jobs_completed += 1;
                            }
                            Err(e) => {
                                tracing::warn!("Failed to submit receipt: {}", e);
                                self.telemetry.record_receipt_failure();
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Job execution failed: {}", e);
                        timer.fail();

                        // Rollback state
                        let mut sm = self.state_manager.write().await;
                        sm.rollback();

                        let mut state = self.state.write().await;
                        state.jobs_failed += 1;
                    }
                }
            } else {
                // No jobs, sleep
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
    }
}

/// Initialize logging
pub fn init_logging(level: Level) {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");
}
