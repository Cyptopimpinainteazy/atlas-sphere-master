//! Rewards engine: per-second accrual and settlement

use crate::reputation::{ReputationRepo, ReputationManager};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PendingReward {
    pub node_id: Uuid,
    pub wallet_address: String,
    pub amount: f64,
    pub last_updated: DateTime<Utc>,
}

/// Simple in-memory rewards engine for accruing per-second drips
pub struct RewardsEngine<R: ReputationRepo> {
    pub repo: Arc<R>,
    // pending rewards per node
    pub pending: Arc<Mutex<HashMap<Uuid, PendingReward>>>,
}

impl<R: ReputationRepo> RewardsEngine<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo, pending: Arc::new(Mutex::new(HashMap::new())) }
    }

    /// Accrue rewards for a node over `secs` seconds using formula described in spec
    /// base_rate is per-second base X3Coin for the GPU class
    pub fn accrue_for_node(&self, node_id: Uuid, wallet: String, base_rate_per_sec: f64, uptime_fraction: f64, success_factor: f64, rep_multiplier: f64, secs: u64) {
        let uptime = uptime_fraction.clamp(0.0, 1.0);
        let success = success_factor.clamp(0.0, 1.0);
        let rep = rep_multiplier.max(0.0);
        let amount = base_rate_per_sec * (uptime) * (success) * rep * (secs as f64);

        let mut g = self.pending.lock().unwrap();
        let entry = g.entry(node_id).or_insert(PendingReward { node_id, wallet_address: wallet.clone(), amount: 0.0, last_updated: Utc::now() });
        entry.amount += amount;
        entry.last_updated = Utc::now();
    }

    /// Get pending amount for wallet
    pub fn pending_for_wallet(&self, wallet: &str) -> f64 {
        let g = self.pending.lock().unwrap();
        g.values().filter(|p| p.wallet_address == wallet).map(|p| p.amount).sum()
    }

    /// Settle pending rewards for a wallet (mark as settled and clear pending)
    pub async fn settle_for_wallet(&self, wallet: &str) {
        // In production: write to reward_settled and produce merkle commit
        let mut g = self.pending.lock().unwrap();
        let keys: Vec<_> = g.iter().filter(|(_, p)| p.wallet_address == wallet).map(|(k, _)| *k).collect();
        for k in keys {
            let p = g.remove(&k).unwrap();
            // Create a slashing event or settled reward entry via repo if needed
            // For now we just drop into repo via reputation event for auditing
            let ev = crate::reputation::ReputationEvent {
                id: 0,
                wallet_address: p.wallet_address.clone(),
                node_id: Some(p.node_id),
                event_type: "rewards_settled".to_string(),
                delta: p.amount,
                prev_reputation: 0.0,
                new_reputation: 0.0,
                evidence_hash: None,
                occurred_at: Utc::now(),
            };
            let _ = self.repo.insert_reputation_event(ev).await;
        }
    }
}
