//! System monitoring for Dream Mining

use crate::{DreamConfig, DreamResult, DreamError};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// System statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStats {
    /// Whether user is currently active
    pub user_active: bool,
    
    /// Seconds since last user activity
    pub idle_time_secs: u64,
    
    /// GPU temperature in Celsius
    pub gpu_temperature: Option<f32>,
    
    /// GPU usage percentage
    pub gpu_usage: Option<f32>,
    
    /// GPU memory usage percentage
    pub gpu_memory_usage: Option<f32>,
    
    /// CPU usage percentage
    pub cpu_usage: Option<f32>,
    
    /// Memory usage percentage
    pub memory_usage: Option<f32>,
    
    /// Whether on battery power
    pub on_battery: bool,
    
    /// Battery level percentage
    pub battery_level: Option<f32>,
    
    /// System load average
    pub load_average: [f64; 3],
}

impl Default for SystemStats {
    fn default() -> Self {
        Self {
            user_active: false,
            idle_time_secs: 0,
            gpu_temperature: None,
            gpu_usage: None,
            gpu_memory_usage: None,
            cpu_usage: None,
            memory_usage: None,
            on_battery: false,
            battery_level: None,
            load_average: [0.0; 3],
        }
    }
}

/// System resource monitor
pub struct SystemMonitor {
    config: DreamConfig,
    stats: Arc<RwLock<SystemStats>>,
    running: std::sync::atomic::AtomicBool,
}

impl SystemMonitor {
    /// Create a new system monitor
    pub fn new(config: &DreamConfig) -> Self {
        Self {
            config: config.clone(),
            stats: Arc::new(RwLock::new(SystemStats::default())),
            running: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Start monitoring
    pub async fn start(&self) -> DreamResult<()> {
        use std::sync::atomic::Ordering;
        
        if self.running.swap(true, Ordering::SeqCst) {
            return Err(DreamError::AlreadyRunning);
        }

        let stats = self.stats.clone();
        let interval = self.config.activity_check_interval_secs;
        let idle_threshold = self.config.idle_threshold_secs;
        let running = Arc::new(std::sync::atomic::AtomicBool::new(true));

        // Spawn monitoring task
        let stats_clone = stats.clone();
        let running_clone = running.clone();
        tokio::spawn(async move {
            let mut sys = sysinfo::System::new_all();
            let mut last_activity = std::time::Instant::now();
            
            loop {
                if !running_clone.load(Ordering::SeqCst) {
                    break;
                }

                // Refresh system info
                sys.refresh_all();

                // Calculate CPU usage
                let cpu_usage = sys.cpus().iter()
                    .map(|cpu| cpu.cpu_usage())
                    .sum::<f32>() / sys.cpus().len() as f32;

                // Calculate memory usage
                let total_memory = sys.total_memory();
                let used_memory = sys.used_memory();
                let memory_usage = if total_memory > 0 {
                    (used_memory as f64 / total_memory as f64 * 100.0) as f32
                } else {
                    0.0
                };

                // Detect user activity (simplified - check CPU spike)
                let user_active = cpu_usage > 30.0;
                if user_active {
                    last_activity = std::time::Instant::now();
                }
                let idle_time = last_activity.elapsed().as_secs();

                // Update stats
                {
                    let mut stats = stats_clone.write().await;
                    stats.cpu_usage = Some(cpu_usage);
                    stats.memory_usage = Some(memory_usage);
                    stats.idle_time_secs = idle_time;
                    stats.user_active = idle_time < idle_threshold;
                    
                    // Note: GPU monitoring requires platform-specific implementations
                    // For now, we'll use placeholder values
                    // In production, use nvml (NVIDIA) or similar APIs
                    stats.gpu_usage = None;
                    stats.gpu_temperature = None;
                    stats.gpu_memory_usage = None;
                    
                    // Battery info (placeholder)
                    stats.on_battery = false;
                    stats.battery_level = Some(100.0);
                }

                tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
            }
        });

        Ok(())
    }

    /// Stop monitoring
    pub fn stop(&self) {
        use std::sync::atomic::Ordering;
        self.running.store(false, Ordering::SeqCst);
    }

    /// Get current system stats
    pub async fn current_stats(&self) -> DreamResult<SystemStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }

    /// Check if system is idle
    pub async fn is_idle(&self) -> DreamResult<bool> {
        let stats = self.stats.read().await;
        Ok(!stats.user_active)
    }

    /// Check if GPU is available for mining
    pub async fn gpu_available(&self) -> DreamResult<bool> {
        let stats = self.stats.read().await;
        
        // Check temperature
        if let Some(temp) = stats.gpu_temperature {
            if temp > self.config.max_gpu_temp {
                return Ok(false);
            }
        }
        
        // Check usage
        if let Some(usage) = stats.gpu_usage {
            if usage > self.config.max_gpu_usage {
                return Ok(false);
            }
        }
        
        Ok(true)
    }

    /// Check if CPU is available for mining
    pub async fn cpu_available(&self) -> DreamResult<bool> {
        let stats = self.stats.read().await;
        
        if let Some(usage) = stats.cpu_usage {
            Ok(usage < self.config.max_cpu_usage)
        } else {
            Ok(true)
        }
    }

    /// Check battery status
    pub async fn battery_ok(&self) -> DreamResult<bool> {
        let stats = self.stats.read().await;
        
        // If on battery and battery mining disabled
        if stats.on_battery && !self.config.mine_on_battery {
            return Ok(false);
        }
        
        // Check battery level
        if let Some(level) = stats.battery_level {
            if level < self.config.min_battery_level {
                return Ok(false);
            }
        }
        
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_stats() {
        let stats = SystemStats::default();
        assert!(!stats.user_active);
        assert_eq!(stats.idle_time_secs, 0);
    }
}
