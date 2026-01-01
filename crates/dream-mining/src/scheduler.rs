//! Task scheduler for Dream Mining

use crate::{DreamConfig, DreamResult, DreamError};
use crate::tasks::{DreamTask, TaskPriority, TaskResult, TaskStatus};
use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Task scheduler statistics
#[derive(Debug, Clone, Default)]
pub struct SchedulerStats {
    pub completed: u64,
    pub pending: u64,
    pub failed: u64,
    pub current_task: Option<String>,
    pub uptime_hours: f64,
}

/// Task queue entry (for priority ordering)
#[derive(Debug)]
struct TaskEntry {
    task_id: Uuid,
    priority: TaskPriority,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl PartialEq for TaskEntry {
    fn eq(&self, other: &Self) -> bool {
        self.task_id == other.task_id
    }
}

impl Eq for TaskEntry {}

impl PartialOrd for TaskEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TaskEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Higher priority first, then older tasks first
        match self.priority.cmp(&other.priority) {
            std::cmp::Ordering::Equal => other.created_at.cmp(&self.created_at),
            other => other,
        }
    }
}

/// Dream Mining task scheduler
pub struct DreamScheduler {
    config: DreamConfig,
    tasks: Arc<RwLock<HashMap<Uuid, DreamTask>>>,
    queue: Arc<RwLock<BinaryHeap<TaskEntry>>>,
    current_task: Arc<RwLock<Option<Uuid>>>,
    stats: Arc<RwLock<SchedulerStats>>,
    start_time: chrono::DateTime<chrono::Utc>,
}

impl DreamScheduler {
    /// Create a new scheduler
    pub fn new(config: &DreamConfig) -> Self {
        Self {
            config: config.clone(),
            tasks: Arc::new(RwLock::new(HashMap::new())),
            queue: Arc::new(RwLock::new(BinaryHeap::new())),
            current_task: Arc::new(RwLock::new(None)),
            stats: Arc::new(RwLock::new(SchedulerStats::default())),
            start_time: chrono::Utc::now(),
        }
    }

    /// Add a new task to the queue
    pub async fn add_task(&self, task: DreamTask) -> DreamResult<Uuid> {
        let id = task.id;
        let priority = task.priority;
        let created_at = task.created_at;

        // Add to tasks map
        {
            let mut tasks = self.tasks.write().await;
            tasks.insert(id, task);
        }

        // Add to priority queue
        {
            let mut queue = self.queue.write().await;
            queue.push(TaskEntry {
                task_id: id,
                priority,
                created_at,
            });
        }

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.pending += 1;
        }

        tracing::info!(task_id = %id, priority = ?priority, "Task added to queue");
        Ok(id)
    }

    /// Get the next task to execute
    pub async fn next_task(&self) -> DreamResult<Option<DreamTask>> {
        // Check if already running a task
        {
            let current = self.current_task.read().await;
            if current.is_some() {
                return Ok(None);
            }
        }

        // Get highest priority task
        let task_id = {
            let mut queue = self.queue.write().await;
            queue.pop().map(|entry| entry.task_id)
        };

        let task_id = match task_id {
            Some(id) => id,
            None => return Ok(None),
        };

        // Get and update task
        let task = {
            let mut tasks = self.tasks.write().await;
            let task = tasks.get_mut(&task_id).ok_or(DreamError::TaskNotFound(task_id))?;
            task.status = TaskStatus::Running;
            task.updated_at = chrono::Utc::now();
            task.clone()
        };

        // Set as current task
        {
            let mut current = self.current_task.write().await;
            *current = Some(task_id);
        }

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.pending = stats.pending.saturating_sub(1);
            stats.current_task = Some(format!("{:?}", task.task_type));
        }

        Ok(Some(task))
    }

    /// Mark a task as completed
    pub async fn complete_task(&self, task_id: Uuid, result: TaskResult) -> DreamResult<()> {
        // Update task
        {
            let mut tasks = self.tasks.write().await;
            let task = tasks.get_mut(&task_id).ok_or(DreamError::TaskNotFound(task_id))?;
            task.status = TaskStatus::Completed;
            task.updated_at = chrono::Utc::now();
            task.progress = 1.0;
        }

        // Clear current task
        {
            let mut current = self.current_task.write().await;
            *current = None;
        }

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.completed += 1;
            stats.current_task = None;
        }

        tracing::info!(task_id = %task_id, "Task completed");
        Ok(())
    }

    /// Mark a task as failed
    pub async fn fail_task(&self, task_id: Uuid, error: String) -> DreamResult<()> {
        // Get task for retry check
        let should_retry = {
            let tasks = self.tasks.read().await;
            let task = tasks.get(&task_id).ok_or(DreamError::TaskNotFound(task_id))?;
            task.retries < task.max_retries
        };

        if should_retry {
            // Retry task
            let task = {
                let mut tasks = self.tasks.write().await;
                let task = tasks.get_mut(&task_id).ok_or(DreamError::TaskNotFound(task_id))?;
                task.retries += 1;
                task.status = TaskStatus::Pending;
                task.updated_at = chrono::Utc::now();
                task.clone()
            };

            // Re-add to queue
            {
                let mut queue = self.queue.write().await;
                queue.push(TaskEntry {
                    task_id,
                    priority: task.priority,
                    created_at: task.created_at,
                });
            }

            tracing::warn!(task_id = %task_id, retries = task.retries, "Task failed, retrying");
        } else {
            // Mark as permanently failed
            {
                let mut tasks = self.tasks.write().await;
                let task = tasks.get_mut(&task_id).ok_or(DreamError::TaskNotFound(task_id))?;
                task.status = TaskStatus::Failed(error.clone());
                task.updated_at = chrono::Utc::now();
            }

            // Update stats
            {
                let mut stats = self.stats.write().await;
                stats.failed += 1;
            }

            tracing::error!(task_id = %task_id, error = %error, "Task permanently failed");
        }

        // Clear current task
        {
            let mut current = self.current_task.write().await;
            *current = None;
        }

        {
            let mut stats = self.stats.write().await;
            stats.current_task = None;
        }

        Ok(())
    }

    /// Pause a running task
    pub async fn pause_task(&self, task_id: Uuid, progress: f64) -> DreamResult<()> {
        // Update task
        {
            let mut tasks = self.tasks.write().await;
            let task = tasks.get_mut(&task_id).ok_or(DreamError::TaskNotFound(task_id))?;
            task.status = TaskStatus::Paused;
            task.updated_at = chrono::Utc::now();
            task.progress = progress;
        }

        // Clear current task
        {
            let mut current = self.current_task.write().await;
            *current = None;
        }

        {
            let mut stats = self.stats.write().await;
            stats.current_task = None;
        }

        tracing::info!(task_id = %task_id, progress = progress, "Task paused");
        Ok(())
    }

    /// Resume a paused task
    pub async fn resume_task(&self, task_id: Uuid) -> DreamResult<()> {
        let task = {
            let mut tasks = self.tasks.write().await;
            let task = tasks.get_mut(&task_id).ok_or(DreamError::TaskNotFound(task_id))?;
            task.status = TaskStatus::Pending;
            task.updated_at = chrono::Utc::now();
            task.clone()
        };

        // Add back to queue
        {
            let mut queue = self.queue.write().await;
            queue.push(TaskEntry {
                task_id,
                priority: task.priority,
                created_at: task.created_at,
            });
        }

        {
            let mut stats = self.stats.write().await;
            stats.pending += 1;
        }

        tracing::info!(task_id = %task_id, "Task resumed");
        Ok(())
    }

    /// Cancel a task
    pub async fn cancel_task(&self, task_id: Uuid) -> DreamResult<()> {
        {
            let mut tasks = self.tasks.write().await;
            let task = tasks.get_mut(&task_id).ok_or(DreamError::TaskNotFound(task_id))?;
            task.status = TaskStatus::Cancelled;
            task.updated_at = chrono::Utc::now();
        }

        // Clear if current task
        {
            let mut current = self.current_task.write().await;
            if *current == Some(task_id) {
                *current = None;
            }
        }

        tracing::info!(task_id = %task_id, "Task cancelled");
        Ok(())
    }

    /// Get a task by ID
    pub async fn get_task(&self, task_id: Uuid) -> DreamResult<DreamTask> {
        let tasks = self.tasks.read().await;
        tasks.get(&task_id).cloned().ok_or(DreamError::TaskNotFound(task_id))
    }

    /// Get all tasks
    pub async fn all_tasks(&self) -> DreamResult<Vec<DreamTask>> {
        let tasks = self.tasks.read().await;
        Ok(tasks.values().cloned().collect())
    }

    /// Get scheduler statistics
    pub async fn stats(&self) -> DreamResult<SchedulerStats> {
        let mut stats = self.stats.read().await.clone();
        
        // Calculate uptime
        let now = chrono::Utc::now();
        let duration = now.signed_duration_since(self.start_time);
        stats.uptime_hours = duration.num_minutes() as f64 / 60.0;
        
        Ok(stats)
    }

    /// Clear completed tasks
    pub async fn clear_completed(&self) -> DreamResult<usize> {
        let mut tasks = self.tasks.write().await;
        let before = tasks.len();
        tasks.retain(|_, task| !matches!(task.status, TaskStatus::Completed));
        let removed = before - tasks.len();
        tracing::info!(removed = removed, "Cleared completed tasks");
        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::{TaskType, ModelTrainingTask};

    #[tokio::test]
    async fn test_task_scheduling() {
        let config = DreamConfig::default();
        let scheduler = DreamScheduler::new(&config);

        // Add a task
        let task = DreamTask::new(
            TaskType::ModelTraining(ModelTrainingTask {
                model_name: "test".to_string(),
                training_data: "data".to_string(),
                epochs: 10,
                batch_size: 32,
                learning_rate: 0.001,
                checkpoint_path: None,
            }),
            TaskPriority::Normal,
        );
        
        let task_id = scheduler.add_task(task).await.unwrap();
        
        // Get next task
        let next = scheduler.next_task().await.unwrap();
        assert!(next.is_some());
        assert_eq!(next.unwrap().id, task_id);
    }

    #[tokio::test]
    async fn test_priority_ordering() {
        let config = DreamConfig::default();
        let scheduler = DreamScheduler::new(&config);

        // Add low priority task first
        let low_task = DreamTask::new(
            TaskType::ModelTraining(ModelTrainingTask {
                model_name: "low".to_string(),
                training_data: "data".to_string(),
                epochs: 10,
                batch_size: 32,
                learning_rate: 0.001,
                checkpoint_path: None,
            }),
            TaskPriority::Low,
        );
        scheduler.add_task(low_task).await.unwrap();

        // Add high priority task
        let high_task = DreamTask::new(
            TaskType::ModelTraining(ModelTrainingTask {
                model_name: "high".to_string(),
                training_data: "data".to_string(),
                epochs: 10,
                batch_size: 32,
                learning_rate: 0.001,
                checkpoint_path: None,
            }),
            TaskPriority::High,
        );
        let high_id = scheduler.add_task(high_task).await.unwrap();

        // High priority should come first
        let next = scheduler.next_task().await.unwrap().unwrap();
        assert_eq!(next.id, high_id);
    }
}
