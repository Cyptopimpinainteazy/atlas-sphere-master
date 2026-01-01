use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ReputationFactors {
    pub uptime_fraction: f64,
    pub success_rate: f64,
    pub watchdog_pass_rate: f64,
    pub performance_consistency: f64,
}

impl ReputationFactors {
    pub fn clamp(&mut self) {
        self.uptime_fraction = self.uptime_fraction.clamp(0.0, 1.0);
        self.success_rate = self.success_rate.clamp(0.0, 1.0);
        self.watchdog_pass_rate = self.watchdog_pass_rate.clamp(0.0, 1.0);
        self.performance_consistency = self.performance_consistency.clamp(0.0, 1.0);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ReputationEvent {
    pub id: i64,
    pub wallet_address: String,
    pub node_id: Option<Uuid>,
    pub event_type: String,
    pub delta: f64,
    pub prev_reputation: f64,
    pub new_reputation: f64,
    pub evidence_hash: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SlashingEvent {
    pub id: i64,
    pub wallet_address: String,
    pub node_id: Option<Uuid>,
    pub severity: f64,
    pub slash_amount: f64,
    pub recurrence_count: i32,
    pub evidence_hash: Option<String>,
    pub occurred_at: DateTime<Utc>,
    pub appeal_status: String,
}

#[async_trait::async_trait]
pub trait ReputationRepo: Send + Sync + 'static {
    async fn insert_reputation_event(&self, ev: ReputationEvent) -> Result<(), String>;
    async fn insert_slashing_event(&self, ev: SlashingEvent) -> Result<(), String>;
    async fn get_reputation_events(&self, wallet: &str) -> Result<Vec<ReputationEvent>, String>;
    async fn get_slashing_events(&self, wallet: &str) -> Result<Vec<SlashingEvent>, String>;
}

#[derive(Clone, Default)]
pub struct InMemoryRepo {
    pub events: Arc<Mutex<Vec<ReputationEvent>>>,
    pub slashes: Arc<Mutex<Vec<SlashingEvent>>>,
}

#[async_trait::async_trait]
impl ReputationRepo for InMemoryRepo {
    async fn insert_reputation_event(&self, ev: ReputationEvent) -> Result<(), String> {
        let mut g = self.events.lock().unwrap();
        g.push(ev);
        Ok(())
    }
    async fn insert_slashing_event(&self, ev: SlashingEvent) -> Result<(), String> {
        let mut g = self.slashes.lock().unwrap();
        g.push(ev);
        Ok(())
    }
    async fn get_reputation_events(&self, wallet: &str) -> Result<Vec<ReputationEvent>, String> {
        let g = self.events.lock().unwrap();
        Ok(g.iter().filter(|e| e.wallet_address == wallet).cloned().collect())
    }
    async fn get_slashing_events(&self, wallet: &str) -> Result<Vec<SlashingEvent>, String> {
        let g = self.slashes.lock().unwrap();
        Ok(g.iter().filter(|e| e.wallet_address == wallet).cloned().collect())
    }
}

pub fn compute_node_reputation(mut f: ReputationFactors, incident_penalty: f64) -> (f64, f64) {
    f.clamp();
    let raw = 0.25 * f.uptime_fraction
        + 0.35 * f.success_rate
        + 0.25 * f.watchdog_pass_rate
        + 0.10 * f.performance_consistency;
    let incident = incident_penalty.clamp(0.0, 1.0);
    let adj = raw * incident;
    (raw, adj)
}

pub fn compute_wallet_reputation(node_reps: &[f64], funding_factor: f64, social_factor: f64) -> f64 {
    if node_reps.is_empty() {
        return 0.5_f64.clamp(0.0, 1.0);
    }
    let mut reps = node_reps.to_vec();
    reps.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = if reps.len() % 2 == 1 {
        reps[reps.len() / 2]
    } else {
        let hi = reps.len() / 2;
        (reps[hi - 1] + reps[hi]) / 2.0
    };
    let f_funding = funding_factor.clamp(0.0, 0.2);
    let f_social = social_factor.clamp(0.0, 0.1);
    let wallet_rep = (median * (1.0 + f_funding + f_social)).clamp(0.0, 1.0);
    wallet_rep
}

pub fn compute_slash_amount(bond: f64, severity: f64, repeat_count: i32, base_scale: f64) -> f64 {
    let severity = severity.clamp(0.0, 1.0);
    let repeat_factor = 1.0 + 0.5 * ((repeat_count - 1).max(0) as f64);
    let base_slash_fraction = severity * base_scale;
    let slash = (bond * base_slash_fraction * repeat_factor).floor();
    slash.min(bond)
}

pub struct ReputationManager<R: ReputationRepo> {
    pub repo: Arc<R>,
}

impl<R: ReputationRepo> ReputationManager<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }
    pub async fn record_reputation_event(&self, ev: ReputationEvent) -> Result<(), String> {
        self.repo.insert_reputation_event(ev).await
    }
    pub async fn record_slashing_event(&self, ev: SlashingEvent) -> Result<(), String> {
        self.repo.insert_slashing_event(ev).await
    }
}

mod pg_impl {
    use super::*;
    use sqlx::PgPool;

    pub struct PgRepo {
        pool: PgPool,
    }

    impl PgRepo {
        pub fn new(pool: PgPool) -> Self {
            Self { pool }
        }
    }

    #[async_trait::async_trait]
    impl ReputationRepo for PgRepo {
        async fn insert_reputation_event(&self, ev: ReputationEvent) -> Result<(), String> {
            let query = r#"INSERT INTO reputation_events(wallet_address,node_id,event_type,delta,prev_reputation,new_reputation,evidence_hash,occurred_at)
                VALUES($1,$2,$3,$4,$5,$6,$7,$8)"#;
            let node_id = ev.node_id.map(|u| u.to_string());
            sqlx::query(query)
                .bind(&ev.wallet_address)
                .bind(node_id)
                .bind(&ev.event_type)
                .bind(ev.delta as f64)
                .bind(ev.prev_reputation as f64)
                .bind(ev.new_reputation as f64)
                .bind(ev.evidence_hash)
                .bind(ev.occurred_at.naive_utc())
                .execute(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
            Ok(())
        }
        async fn insert_slashing_event(&self, ev: SlashingEvent) -> Result<(), String> {
            let query = r#"INSERT INTO slashing_events(wallet_address,node_id,severity,slash_amount,recurrence_count,evidence_hash,occurred_at,appeal_status)
                VALUES($1,$2,$3,$4,$5,$6,$7,$8)"#;
            let node_id = ev.node_id.map(|u| u.to_string());
            sqlx::query(query)
                .bind(&ev.wallet_address)
                .bind(node_id)
                .bind(ev.severity as f64)
                .bind(ev.slash_amount as f64)
                .bind(ev.recurrence_count)
                .bind(ev.evidence_hash)
                .bind(ev.occurred_at.naive_utc())
                .bind(&ev.appeal_status)
                .execute(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
            Ok(())
        }
        async fn get_reputation_events(&self, wallet: &str) -> Result<Vec<ReputationEvent>, String> {
            let query = r#"SELECT id, wallet_address, node_id, event_type, delta, prev_reputation, new_reputation, evidence_hash, occurred_at FROM reputation_events WHERE wallet_address = $1"#;
            let rows = sqlx::query_as::<_, (i32, String, Option<String>, String, f64, f64, f64, Option<String>, chrono::DateTime<Utc>)>(query)
                .bind(wallet)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
            let evs = rows.into_iter().map(|r| ReputationEvent {
                id: r.0 as i64,
                wallet_address: r.1,
                node_id: r.2.and_then(|s| uuid::Uuid::parse_str(&s).ok()),
                event_type: r.3,
                delta: r.4,
                prev_reputation: r.5,
                new_reputation: r.6,
                evidence_hash: r.7,
                occurred_at: r.8,
            }).collect();
            Ok(evs)
        }
        async fn get_slashing_events(&self, wallet: &str) -> Result<Vec<SlashingEvent>, String> {
            let query = r#"SELECT id, wallet_address, node_id, severity, slash_amount, recurrence_count, evidence_hash, occurred_at, appeal_status FROM slashing_events WHERE wallet_address = $1"#;
            let rows = sqlx::query_as::<_, (i32, String, Option<String>, f64, f64, i32, Option<String>, chrono::DateTime<Utc>, Option<String>)>(query)
                .bind(wallet)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
            let evs = rows.into_iter().map(|r| SlashingEvent {
                id: r.0 as i64,
                wallet_address: r.1,
                node_id: r.2.and_then(|s| uuid::Uuid::parse_str(&s).ok()),
                severity: r.3,
                slash_amount: r.4,
                recurrence_count: r.5,
                evidence_hash: r.6,
                occurred_at: r.7,
                appeal_status: r.8.unwrap_or_else(|| "none".to_string()),
            }).collect();
            Ok(evs)
        }
    }
}

pub fn new_pg_repo(pool: sqlx::PgPool) -> std::sync::Arc<dyn ReputationRepo> {
    std::sync::Arc::new(pg_impl::PgRepo::new(pool))
}

pub use pg_impl::PgRepo;

#[cfg(test)]
#[tokio::test]
async fn test_pgrepo_reputation_events_roundtrip() {
    if std::env::var("DATABASE_URL").is_err() {
        return;
    }

    let database_url = std::env::var("DATABASE_URL").unwrap();
    let pool = sqlx::PgPool::connect(&database_url).await.expect("could not connect to postgres for test");

    // ensure tables exist
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
    sqlx::query(create_reputation).execute(&pool).await.expect("migration failed");
    sqlx::query(create_slashing).execute(&pool).await.expect("migration failed");

    let repo = pg_impl::PgRepo::new(pool);

    let ev = ReputationEvent {
        id: 0,
        wallet_address: "0xroundtrip".to_string(),
        node_id: None,
        event_type: "integration_test".to_string(),
        delta: 0.12,
        prev_reputation: 0.5,
        new_reputation: 0.62,
        evidence_hash: Some("h1".to_string()),
        occurred_at: Utc::now(),
    };

    repo.insert_reputation_event(ev.clone()).await.expect("insert failed");
    let found = repo.get_reputation_events("0xroundtrip").await.expect("query failed");
    assert!(found.len() >= 1);
    assert_eq!(found[0].wallet_address, "0xroundtrip");
}

#[cfg(test)]
#[tokio::test]
async fn test_slashing_appeal_lifecycle_and_recurrence() {
    if std::env::var("DATABASE_URL").is_err() {
        return;
    }
    let database_url = std::env::var("DATABASE_URL").unwrap();
    let pool = sqlx::PgPool::connect(&database_url).await.expect("could not connect to postgres for test");
    // Ensure schema
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
    sqlx::query(create_slashing).execute(&pool).await.expect("migration failed");

    let repo = pg_impl::PgRepo::new(pool.clone());
    // Insert a pending appeal
    let now = Utc::now();
    let s = SlashingEvent { id: 0, wallet_address: "0xappeal".to_string(), node_id: None, severity: 0.4, slash_amount: 4.0, recurrence_count: 1, evidence_hash: None, occurred_at: now, appeal_status: "pending".to_string() };
    repo.insert_slashing_event(s).await.expect("insert failed");

    // Update appeal status via SQL (simulate appeal resolution)
    sqlx::query("UPDATE slashing_events SET appeal_status = $1 WHERE wallet_address = $2")
        .bind("accepted")
        .bind("0xappeal")
        .execute(&pool)
        .await
        .expect("update failed");

    let events = repo.get_slashing_events("0xappeal").await.expect("query failed");
    assert!(events.len() >= 1);
    assert_eq!(events[0].appeal_status, "accepted");

    // Recurrence math: ensure repeat increases slash
    let s1 = compute_slash_amount(100.0, 0.5, 1, 0.5);
    let s2 = compute_slash_amount(100.0, 0.5, 3, 0.5);
    assert!(s2 > s1);
}

#[test]
fn test_compute_wallet_reputation_behaviors() {
    // Empty nodes => fallback 0.5
    let w = compute_wallet_reputation(&[], 0.0, 0.0);
    assert!((w - 0.5).abs() < 1e-9);

    // With nodes and funding/social factors
    let nodes = vec![0.9, 0.8, 0.95];
    let w2 = compute_wallet_reputation(&nodes, 0.1, 0.05);
    assert!(w2 > 0.9);
}
