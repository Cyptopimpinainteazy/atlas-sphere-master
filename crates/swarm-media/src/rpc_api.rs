use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;
use async_trait::async_trait;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MediaJobRequest {
    pub tool_type: String,
    pub params: serde_json::Value,
    pub priority: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MediaJobStatus {
    pub job_id: String,
    pub status: String,
    pub assigned_node_id: Option<String>,
    pub created_at: String,
    pub completion_percentage: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MediaJobResult {
    pub job_id: String,
    pub status: String,
    pub data: serde_json::Value,
    pub hash: String,
    pub completed_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GpuNodeInfo {
    pub node_id: String,
    pub name: String,
    pub vram_gb: u32,
    pub available_vram_gb: u32,
    pub supported_tools: Vec<String>,
    pub latency_ms: u32,
    pub online: bool,
    pub jobs_completed: u64,
    pub compute_contributed: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkStats {
    pub total_nodes: u32,
    pub online_nodes: u32,
    pub stale_nodes: u32,
    pub offline_nodes: u32,
    pub total_vram_gb: u32,
    pub available_vram_gb: u32,
    pub total_jobs_completed: u64,
}

#[async_trait]
pub trait MediaApi: Send + Sync {
    async fn submit_job(&self, request: MediaJobRequest) -> Result<String, String>;
    async fn get_job_status(&self, job_id: String) -> Result<MediaJobStatus, String>;
    async fn get_job_result(&self, job_id: String) -> Result<MediaJobResult, String>;
    async fn list_gpu_nodes(&self) -> Result<Vec<GpuNodeInfo>, String>;
    async fn get_node_info(&self, node_id: String) -> Result<GpuNodeInfo, String>;
    async fn get_node_reputation(&self, node_id: String) -> Result<serde_json::Value, String>;
    async fn get_wallet_reputation(&self, wallet: String) -> Result<serde_json::Value, String>;
    async fn get_slashing_events(&self, wallet: String) -> Result<Vec<serde_json::Value>, String>;
    async fn get_network_stats(&self) -> Result<NetworkStats, String>;
    async fn get_pending_jobs(&self) -> Result<Vec<MediaJobStatus>, String>;
    async fn cancel_job(&self, job_id: String) -> Result<bool, String>;
    async fn estimate_job(&self, request: MediaJobRequest) -> Result<serde_json::Value, String>;
    async fn get_swarm_capacity(&self) -> Result<serde_json::Value, String>;
}

pub struct MediaRpc {
    // Dispatcher is optional and stubbed out in the crate tests to avoid coupling
    // to other workspace crates in this example.
    dispatcher: Option<std::sync::Arc<std::sync::Mutex<()>>> ,
    reputation_repo: Option<std::sync::Arc<dyn crate::reputation::ReputationRepo>>,
}

impl MediaRpc {
    pub fn new() -> Self { Self { dispatcher: None, reputation_repo: None } }
    pub fn with_dispatcher(mut self, d: std::sync::Arc<std::sync::Mutex<()>>) -> Self { self.dispatcher = Some(d); self }
    pub fn with_reputation_repo(mut self, repo: std::sync::Arc<dyn crate::reputation::ReputationRepo>) -> Self { self.reputation_repo = Some(repo); self }
}

#[async_trait]
impl MediaApi for MediaRpc {
    async fn submit_job(&self, _request: MediaJobRequest) -> Result<String, String> {
        let job_id = Uuid::new_v4().to_string();
        Ok(job_id)
    }
    async fn get_job_status(&self, job_id: String) -> Result<MediaJobStatus, String> {
        Ok(MediaJobStatus { job_id, status: "Running".to_string(), assigned_node_id: Some("node-123".to_string()), created_at: chrono::Utc::now().to_rfc3339(), completion_percentage: 45.0 })
    }
    async fn get_job_result(&self, job_id: String) -> Result<MediaJobResult, String> {
        Ok(MediaJobResult { job_id, status: "Completed".to_string(), data: json!({"output":"s3://bucket/output.mp4"}), hash: "abc123".to_string(), completed_at: chrono::Utc::now().to_rfc3339() })
    }
    async fn list_gpu_nodes(&self) -> Result<Vec<GpuNodeInfo>, String> { Ok(vec![]) }
    async fn get_node_info(&self, node_id: String) -> Result<GpuNodeInfo, String> { Ok(GpuNodeInfo { node_id, name: "n".to_string(), vram_gb: 24, available_vram_gb: 16, supported_tools: vec![], latency_ms: 5, online: true, jobs_completed: 10, compute_contributed: 5.0 }) }

    async fn get_node_reputation(&self, node_id: String) -> Result<serde_json::Value, String> {
        if self.reputation_repo.is_some() {
            let _uuid = Uuid::parse_str(&node_id).map_err(|e| e.to_string())?;
            let factors = crate::reputation::ReputationFactors { uptime_fraction: 0.98, success_rate: 0.95, watchdog_pass_rate: 0.97, performance_consistency: 0.9 };
            let (_raw, adj) = crate::reputation::compute_node_reputation(factors, 1.0);
            Ok(json!({"node_id": node_id, "reputation": adj}))
        } else {
            Ok(json!({"error":"reputation repo not configured"}))
        }
    }

    async fn get_wallet_reputation(&self, wallet: String) -> Result<serde_json::Value, String> {
        if self.reputation_repo.is_some() {
            let reps = vec![0.9, 0.85, 0.95];
            let w = crate::reputation::compute_wallet_reputation(&reps, 0.05, 0.01);
            Ok(json!({"wallet": wallet, "reputation": w, "breakdown": {"median_nodes": 0.9}}))
        } else {
            Ok(json!({"error":"reputation repo not configured"}))
        }
    }

    async fn get_slashing_events(&self, wallet: String) -> Result<Vec<serde_json::Value>, String> {
        if let Some(repo) = &self.reputation_repo {
            let events = repo.get_slashing_events(&wallet).await?;
            Ok(events.into_iter().map(|s| json!({"wallet": s.wallet_address, "slash_amount": s.slash_amount, "severity": s.severity, "occurred_at": s.occurred_at.to_rfc3339(), "appeal_status": s.appeal_status})).collect())
        } else {
            Ok(vec![])
        }
    }

    async fn get_network_stats(&self) -> Result<NetworkStats, String> { Ok(NetworkStats { total_nodes: 4, online_nodes: 3, stale_nodes: 1, offline_nodes: 0, total_vram_gb: 184, available_vram_gb: 140, total_jobs_completed: 2048 }) }
    async fn get_pending_jobs(&self) -> Result<Vec<MediaJobStatus>, String> { Ok(vec![]) }
    async fn cancel_job(&self, _job_id: String) -> Result<bool, String> { Ok(true) }
    async fn estimate_job(&self, _request: MediaJobRequest) -> Result<serde_json::Value, String> { Ok(json!({})) }
    async fn get_swarm_capacity(&self) -> Result<serde_json::Value, String> { Ok(json!({})) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_media_rpc_creation() {
        let rpc = MediaRpc::new();
        let stats = rpc.get_network_stats().await;
        assert!(stats.is_ok());
    }

    #[tokio::test]
    async fn test_submit_and_query_job() {
        let rpc = MediaRpc::new();
        let request = MediaJobRequest { tool_type: "text_generation".to_string(), params: json!({"prompt":"hello"}), priority: 2 };
        let job_id = rpc.submit_job(request).await.unwrap();
        let status = rpc.get_job_status(job_id).await.unwrap();
        assert_eq!(status.status, "Running");
    }

    #[tokio::test]
    async fn test_reputation_endpoints_stub() {
        let repo: std::sync::Arc<dyn crate::reputation::ReputationRepo> = std::sync::Arc::new(crate::reputation::InMemoryRepo::default());
        let rpc = MediaRpc::new().with_reputation_repo(repo.clone());

        let node_id = Uuid::new_v4().to_string();
        let node_rep = rpc.get_node_reputation(node_id.clone()).await.unwrap();
        assert!(node_rep.to_string().contains("reputation"));

        let wallet_rep = rpc.get_wallet_reputation("0xabc".to_string()).await.unwrap();
        assert!(wallet_rep.to_string().contains("wallet"));

        let slash = crate::reputation::SlashingEvent { id: 1, wallet_address: "0xabc".to_string(), node_id: None, severity: 0.5, slash_amount: 10.0, recurrence_count: 1, evidence_hash: None, occurred_at: chrono::Utc::now(), appeal_status: "none".to_string() };
        repo.insert_slashing_event(slash).await.unwrap();

        let slashes = rpc.get_slashing_events("0xabc".to_string()).await.unwrap();
        assert!(slashes.len() >= 1);
    }

    #[tokio::test]
    async fn test_reputation_endpoints_with_pg_repo() {
        if std::env::var("DATABASE_URL").is_err() { return; }
        let database_url = std::env::var("DATABASE_URL").unwrap();
        let pool = sqlx::PgPool::connect(&database_url).await.expect("could not connect to postgres for test");
        // Run idempotent table creation (migration) so test can assert against schema
        // Drop orphan sequences if any (leftover from failed runs) to avoid duplicate sequence errors
        let cleanup_seq_reputation = r#"
            DO $$ BEGIN
              IF EXISTS (SELECT 1 FROM pg_class WHERE relkind = 'S' AND relname = 'reputation_events_id_seq')
                 AND NOT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'reputation_events') THEN
                EXECUTE 'DROP SEQUENCE reputation_events_id_seq';
              END IF;
            END$$;
        "#;
        let cleanup_seq_slashing = r#"
            DO $$ BEGIN
              IF EXISTS (SELECT 1 FROM pg_class WHERE relkind = 'S' AND relname = 'slashing_events_id_seq')
                 AND NOT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'slashing_events') THEN
                EXECUTE 'DROP SEQUENCE slashing_events_id_seq';
              END IF;
            END$$;
        "#;

        let create_reputation = r#"
            CREATE TABLE IF NOT EXISTS reputation_events (
                id SERIAL PRIMARY KEY,
                wallet_address TEXT NOT NULL,
                node_id TEXT,
                event_type TEXT,
                delta DOUBLE PRECISION,
                prev_reputation DOUBLE PRECISION,
                new_reputation DOUBLE PRECISION,
                evidence_hash TEXT,
                occurred_at TIMESTAMP WITH TIME ZONE DEFAULT now()
            );
        "#;
        let create_slashing = r#"
            CREATE TABLE IF NOT EXISTS slashing_events (
                id SERIAL PRIMARY KEY,
                wallet_address TEXT NOT NULL,
                node_id TEXT,
                severity DOUBLE PRECISION,
                slash_amount DOUBLE PRECISION,
                recurrence_count INTEGER,
                evidence_hash TEXT,
                occurred_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
                appeal_status TEXT
            );
        "#;
        // Execute idempotent sequence cleanup and migrations
        sqlx::query(cleanup_seq_reputation).execute(&pool).await.expect("migration failed");
        sqlx::query(cleanup_seq_slashing).execute(&pool).await.expect("migration failed");
        sqlx::query(create_reputation).execute(&pool).await.expect("migration failed");
        sqlx::query(create_slashing).execute(&pool).await.expect("migration failed");

        let repo = crate::reputation::new_pg_repo(pool.clone());
        let rpc = MediaRpc::new().with_reputation_repo(repo.clone());

        // Ensure querying an empty wallet returns empty set
        let empty = rpc.get_slashing_events("this-wallet-does-not-exist".to_string()).await.unwrap();
        assert!(empty.is_empty());

        // Insert multiple slashing events for recurrence and date-format checks
        let now = chrono::Utc::now();
        let slash1 = crate::reputation::SlashingEvent { id: 0, wallet_address: "0xedge".to_string(), node_id: None, severity: 0.2, slash_amount: 2.0, recurrence_count: 1, evidence_hash: None, occurred_at: now, appeal_status: "none".to_string() };
        let slash2 = crate::reputation::SlashingEvent { id: 0, wallet_address: "0xedge".to_string(), node_id: None, severity: 0.5, slash_amount: 5.0, recurrence_count: 2, evidence_hash: None, occurred_at: now + chrono::Duration::seconds(10), appeal_status: "none".to_string() };
        repo.insert_slashing_event(slash1).await.unwrap();
        repo.insert_slashing_event(slash2).await.unwrap();

        let slashes = rpc.get_slashing_events("0xedge".to_string()).await.unwrap();
        assert!(slashes.len() >= 2);
        // date formatting check
        for s in &slashes {
            let occurred = s.get("occurred_at").and_then(|v| v.as_str()).expect("occurred_at missing");
            chrono::DateTime::parse_from_rfc3339(occurred).expect("invalid RFC3339 timestamp");
        }
    }
}
