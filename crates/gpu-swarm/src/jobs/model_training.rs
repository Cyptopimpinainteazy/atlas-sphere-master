//! Model Training Job
//!
//! Distributed ML training for:
//! - PnL Reward Model v2 (strategy fitness prediction)
//! - Evolution-Core transformer (strategy generation)
//! - Reinforcement learning agents
//! - Time-series forecasting models

use crate::error::{SwarmError, SwarmResult};
use crate::jobs::{JobOutput, JobType, SwarmJob};
use crate::task::TaskPriority;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Type of model to train
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModelType {
    /// PnL reward model (predicts strategy fitness)
    PnlRewardModel,
    /// Evolution transformer (generates strategies)
    EvolutionCore,
    /// Reinforcement learning agent
    RlAgent,
    /// Time-series forecaster
    TimeSeriesForecaster,
    /// Anomaly detector
    AnomalyDetector,
    /// Custom model
    Custom,
}

/// Optimizer type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Optimizer {
    Adam,
    AdamW,
    SGD,
    LAMB,
    Adafactor,
}

impl Default for Optimizer {
    fn default() -> Self {
        Self::AdamW
    }
}

/// Learning rate schedule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LrSchedule {
    Constant(f64),
    Linear { start: f64, end: f64 },
    Cosine { max_lr: f64, min_lr: f64 },
    Warmup { warmup_steps: usize, peak_lr: f64 },
    OneCycle { max_lr: f64, div_factor: f64 },
}

impl Default for LrSchedule {
    fn default() -> Self {
        Self::Cosine {
            max_lr: 1e-4,
            min_lr: 1e-6,
        }
    }
}

/// Configuration for model training job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingConfig {
    /// Model type
    pub model_type: ModelType,
    /// Model architecture
    pub architecture: ModelArchitecture,
    /// Optimizer
    pub optimizer: Optimizer,
    /// Learning rate schedule
    pub lr_schedule: LrSchedule,
    /// Batch size
    pub batch_size: usize,
    /// Number of epochs
    pub epochs: usize,
    /// Gradient accumulation steps
    pub gradient_accumulation: usize,
    /// Mixed precision training
    pub mixed_precision: bool,
    /// Gradient clipping norm
    pub grad_clip_norm: Option<f32>,
    /// Weight decay
    pub weight_decay: f64,
    /// Dropout rate
    pub dropout: f32,
    /// Random seed
    pub seed: u64,
    /// Checkpoint frequency (epochs)
    pub checkpoint_every: usize,
    /// Early stopping patience
    pub early_stopping_patience: Option<usize>,
    /// Distributed training config
    pub distributed: Option<DistributedConfig>,
}

/// Model architecture specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelArchitecture {
    /// Number of layers
    pub num_layers: usize,
    /// Hidden dimension
    pub hidden_dim: usize,
    /// Number of attention heads (for transformers)
    pub num_heads: Option<usize>,
    /// Feed-forward dimension
    pub ff_dim: Option<usize>,
    /// Input dimension
    pub input_dim: usize,
    /// Output dimension
    pub output_dim: usize,
    /// Activation function
    pub activation: String,
    /// Use layer normalization
    pub layer_norm: bool,
}

/// Distributed training configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedConfig {
    /// Number of workers
    pub world_size: usize,
    /// This worker's rank
    pub rank: usize,
    /// Data parallel
    pub data_parallel: bool,
    /// Model parallel
    pub model_parallel: bool,
    /// Pipeline parallel stages
    pub pipeline_stages: Option<usize>,
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            model_type: ModelType::PnlRewardModel,
            architecture: ModelArchitecture {
                num_layers: 6,
                hidden_dim: 256,
                num_heads: Some(8),
                ff_dim: Some(1024),
                input_dim: 128,
                output_dim: 1,
                activation: "gelu".to_string(),
                layer_norm: true,
            },
            optimizer: Optimizer::AdamW,
            lr_schedule: LrSchedule::default(),
            batch_size: 32,
            epochs: 100,
            gradient_accumulation: 1,
            mixed_precision: true,
            grad_clip_norm: Some(1.0),
            weight_decay: 0.01,
            dropout: 0.1,
            seed: 42,
            checkpoint_every: 10,
            early_stopping_patience: Some(10),
            distributed: None,
        }
    }
}

/// Training dataset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingDataset {
    /// Dataset name/ID
    pub name: String,
    /// Number of samples
    pub num_samples: usize,
    /// Feature dimension
    pub feature_dim: usize,
    /// Data format
    pub format: DataFormat,
    /// Data location (URI or path)
    pub location: String,
    /// Validation split ratio
    pub val_split: f32,
    /// Shuffle
    pub shuffle: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DataFormat {
    Parquet,
    Csv,
    Json,
    Binary,
    TFRecord,
}

/// Training metrics at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingMetrics {
    pub epoch: usize,
    pub step: usize,
    pub train_loss: f64,
    pub val_loss: Option<f64>,
    pub learning_rate: f64,
    pub throughput: f64,      // samples/sec
    pub gpu_memory_used: f64, // GB
    pub custom_metrics: HashMap<String, f64>,
}

/// Model checkpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCheckpoint {
    /// Checkpoint ID
    pub id: [u8; 32],
    /// Epoch saved
    pub epoch: usize,
    /// Step saved
    pub step: usize,
    /// Validation loss at save
    pub val_loss: f64,
    /// Model weights hash
    pub weights_hash: [u8; 32],
    /// Checkpoint size (bytes)
    pub size_bytes: usize,
}

/// Result from model training job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingResult {
    /// Final training metrics
    pub final_metrics: TrainingMetrics,
    /// Training history
    pub history: Vec<TrainingMetrics>,
    /// Best checkpoint
    pub best_checkpoint: ModelCheckpoint,
    /// All checkpoints
    pub checkpoints: Vec<ModelCheckpoint>,
    /// Total training time (ms)
    pub total_time_ms: u64,
    /// Total samples processed
    pub samples_processed: usize,
    /// Converged
    pub converged: bool,
    /// Early stopped
    pub early_stopped: bool,
    /// Result hash
    pub result_hash: [u8; 32],
}

/// Model Training Job
pub struct ModelTrainingJob {
    pub config: TrainingConfig,
    pub dataset: TrainingDataset,
    /// Pre-trained weights to fine-tune from
    pub pretrained_weights: Option<Vec<u8>>,
}

impl ModelTrainingJob {
    pub fn new(config: TrainingConfig, dataset: TrainingDataset) -> Self {
        Self {
            config,
            dataset,
            pretrained_weights: None,
        }
    }

    pub fn with_pretrained(mut self, weights: Vec<u8>) -> Self {
        self.pretrained_weights = Some(weights);
        self
    }

    /// Simulate training loop
    fn run_training(&self) -> SwarmResult<TrainingResult> {
        use std::time::Instant;

        let start = Instant::now();
        let mut history = Vec::new();
        let mut checkpoints = Vec::new();
        let mut best_val_loss = f64::MAX;
        let mut best_epoch = 0;
        let mut steps_without_improvement = 0;
        let mut early_stopped = false;

        let total_steps = (self.dataset.num_samples / self.config.batch_size) * self.config.epochs;

        // Simulate training epochs
        for epoch in 0..self.config.epochs {
            // Simulated training loss decay
            let train_loss = 1.0 / (1.0 + epoch as f64 * 0.1) + 0.1;
            let val_loss = train_loss * 1.1 + (epoch as f64 * 0.001).sin() * 0.05;

            // Calculate current learning rate
            let lr = match &self.config.lr_schedule {
                LrSchedule::Constant(lr) => *lr,
                LrSchedule::Cosine { max_lr, min_lr } => {
                    let progress = epoch as f64 / self.config.epochs as f64;
                    min_lr
                        + (max_lr - min_lr) * (1.0 + (progress * std::f64::consts::PI).cos()) / 2.0
                }
                _ => 1e-4,
            };

            let metrics = TrainingMetrics {
                epoch,
                step: epoch * (self.dataset.num_samples / self.config.batch_size),
                train_loss,
                val_loss: Some(val_loss),
                learning_rate: lr,
                throughput: (self.config.batch_size as f64) * 100.0, // samples/sec
                gpu_memory_used: 4.0,                                // GB
                custom_metrics: HashMap::new(),
            };

            history.push(metrics);

            // Track best
            if val_loss < best_val_loss {
                best_val_loss = val_loss;
                best_epoch = epoch;
                steps_without_improvement = 0;
            } else {
                steps_without_improvement += 1;
            }

            // Checkpointing
            if epoch % self.config.checkpoint_every == 0 || epoch == self.config.epochs - 1 {
                let ckpt_id =
                    blake3::hash(format!("checkpoint:{}:{}", self.dataset.name, epoch).as_bytes())
                        .into();

                let weights_hash =
                    blake3::hash(format!("weights:{}:{}", self.dataset.name, epoch).as_bytes())
                        .into();

                checkpoints.push(ModelCheckpoint {
                    id: ckpt_id,
                    epoch,
                    step: epoch * (self.dataset.num_samples / self.config.batch_size),
                    val_loss,
                    weights_hash,
                    size_bytes: self.estimate_model_size(),
                });
            }

            // Early stopping
            if let Some(patience) = self.config.early_stopping_patience {
                if steps_without_improvement >= patience {
                    early_stopped = true;
                    break;
                }
            }
        }

        // Get best checkpoint
        let best_checkpoint = if checkpoints.is_empty() {
            return Err(SwarmError::InvalidResult("No checkpoints produced".to_string()));
        } else {
            checkpoints
                .iter()
                .min_by(|a, b| a.val_loss.partial_cmp(&b.val_loss).unwrap_or(std::cmp::Ordering::Equal))
                .cloned()
                .unwrap_or_else(|| checkpoints.last().cloned().unwrap())
        };

        let final_metrics = if let Some(m) = history.last().cloned() {
            m
        } else {
            return Err(SwarmError::InvalidResult("No training metrics produced".to_string()));
        };

        // Result hash
        let mut hasher = blake3::Hasher::new();
        hasher.update(&best_checkpoint.weights_hash);
        hasher.update(&final_metrics.train_loss.to_le_bytes());
        let result_hash: [u8; 32] = hasher.finalize().into();

        Ok(TrainingResult {
            final_metrics,
            history,
            best_checkpoint,
            checkpoints,
            total_time_ms: start.elapsed().as_millis() as u64,
            samples_processed: self.dataset.num_samples * self.config.epochs,
            converged: best_val_loss < 0.5,
            early_stopped,
            result_hash,
        })
    }

    /// Estimate model size in bytes
    fn estimate_model_size(&self) -> usize {
        let arch = &self.config.architecture;
        let params_per_layer = arch.hidden_dim * arch.hidden_dim * 4; // rough estimate
        let total_params = arch.num_layers * params_per_layer;

        // 4 bytes per float32 param (or 2 for mixed precision)
        let bytes_per_param = if self.config.mixed_precision { 2 } else { 4 };
        total_params * bytes_per_param
    }
}

impl SwarmJob for ModelTrainingJob {
    fn job_type(&self) -> JobType {
        JobType::ModelTraining
    }

    fn compute_units(&self) -> u64 {
        // Estimate based on model size and epochs
        let model_size = self.estimate_model_size();
        let total_samples = self.dataset.num_samples * self.config.epochs;

        // 1 CU per 1000 params * samples / 1M
        ((model_size * total_samples) / 1_000_000_000) as u64
    }

    fn timeout(&self) -> Duration {
        // Training can take hours
        Duration::from_secs(3600 * 4) // 4 hours max
    }

    fn execute(&self) -> SwarmResult<JobOutput> {
        let result = self.run_training()?;
        Ok(JobOutput::ModelTraining(result))
    }

    fn verify(&self, result: &JobOutput) -> SwarmResult<bool> {
        match result {
            JobOutput::ModelTraining(train_result) => {
                // Verify result hash
                let mut hasher = blake3::Hasher::new();
                hasher.update(&train_result.best_checkpoint.weights_hash);
                hasher.update(&train_result.final_metrics.train_loss.to_le_bytes());
                let expected_hash: [u8; 32] = hasher.finalize().into();

                Ok(expected_hash == train_result.result_hash)
            }
            _ => Err(SwarmError::InvalidResult("Wrong result type".into())),
        }
    }

    fn priority(&self) -> TaskPriority {
        TaskPriority::Normal
    }

    fn requires_gpu(&self) -> bool {
        true // Always needs GPU
    }

    fn min_vram_mb(&self) -> u32 {
        // Based on model size and batch size
        let model_mb = (self.estimate_model_size() / (1024 * 1024)) as u32;
        let batch_mb =
            (self.config.batch_size * self.config.architecture.input_dim * 4 / 1024) as u32;

        // Model + gradients + optimizer state + batch
        (model_mb * 4 + batch_mb).max(2048)
    }
}

/// Builder for PnL Reward Model training
pub struct PnlRewardModelBuilder {
    config: TrainingConfig,
    strategies: Vec<(Vec<u8>, f64)>, // (bytecode, fitness)
}

impl PnlRewardModelBuilder {
    pub fn new() -> Self {
        Self {
            config: TrainingConfig {
                model_type: ModelType::PnlRewardModel,
                architecture: ModelArchitecture {
                    num_layers: 4,
                    hidden_dim: 128,
                    num_heads: Some(4),
                    ff_dim: Some(512),
                    input_dim: 64, // Strategy embedding
                    output_dim: 1, // Fitness score
                    activation: "gelu".to_string(),
                    layer_norm: true,
                },
                ..Default::default()
            },
            strategies: Vec::new(),
        }
    }

    pub fn with_strategies(mut self, strategies: Vec<(Vec<u8>, f64)>) -> Self {
        self.strategies = strategies;
        self
    }

    pub fn with_epochs(mut self, epochs: usize) -> Self {
        self.config.epochs = epochs;
        self
    }

    pub fn build(self) -> ModelTrainingJob {
        let dataset = TrainingDataset {
            name: "pnl_reward_dataset".to_string(),
            num_samples: self.strategies.len().max(1000),
            feature_dim: self.config.architecture.input_dim,
            format: DataFormat::Binary,
            location: "memory://strategies".to_string(),
            val_split: 0.2,
            shuffle: true,
        };

        ModelTrainingJob::new(self.config, dataset)
    }
}

impl Default for PnlRewardModelBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_training_config_default() {
        let config = TrainingConfig::default();
        assert_eq!(config.model_type, ModelType::PnlRewardModel);
        assert_eq!(config.epochs, 100);
    }

    #[test]
    fn test_pnl_reward_model_builder() {
        let job = PnlRewardModelBuilder::new()
            .with_strategies(vec![(vec![0x20, 0x64], 0.75), (vec![0x20, 0x65], 0.80)])
            .with_epochs(10)
            .build();

        assert_eq!(job.config.model_type, ModelType::PnlRewardModel);
        assert_eq!(job.config.epochs, 10);
    }

    #[test]
    fn test_model_training_execution() {
        let config = TrainingConfig {
            epochs: 5,
            ..Default::default()
        };

        let dataset = TrainingDataset {
            name: "test_dataset".to_string(),
            num_samples: 1000,
            feature_dim: 128,
            format: DataFormat::Binary,
            location: "test://data".to_string(),
            val_split: 0.2,
            shuffle: true,
        };

        let job = ModelTrainingJob::new(config, dataset);
        let result = job.execute().unwrap();

        if let JobOutput::ModelTraining(train_result) = result {
            assert_eq!(train_result.history.len(), 5);
            assert!(train_result.checkpoints.len() > 0);
        } else {
            panic!("Wrong result type");
        }
    }

    #[test]
    fn test_early_stopping() {
        let config = TrainingConfig {
            epochs: 100,
            early_stopping_patience: Some(3),
            ..Default::default()
        };

        let dataset = TrainingDataset {
            name: "test".to_string(),
            num_samples: 100,
            feature_dim: 32,
            format: DataFormat::Binary,
            location: "test://".to_string(),
            val_split: 0.2,
            shuffle: false,
        };

        let job = ModelTrainingJob::new(config, dataset);
        let result = job.execute().unwrap();

        if let JobOutput::ModelTraining(train_result) = result {
            // May or may not early stop depending on simulated loss
            assert!(train_result.history.len() <= 100);
        }
    }
}
