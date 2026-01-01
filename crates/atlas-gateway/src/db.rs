//! Database connection and queries.

use crate::config::DatabaseConfig;
use crate::error::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::FromRow;
use std::time::Duration;
use tracing::info;

/// Database connection pool wrapper.
#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

// ============================================================================
// Models
// ============================================================================

/// Block data.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Block {
    pub number: i64,
    pub hash: String,
    pub parent_hash: String,
    pub state_root: String,
    pub extrinsics_root: String,
    pub timestamp: DateTime<Utc>,
    pub author: Option<String>,
    pub extrinsic_count: i32,
    pub event_count: i32,
    pub created_at: DateTime<Utc>,
}

/// Extrinsic data.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Extrinsic {
    pub id: i64,
    pub block_number: i64,
    pub extrinsic_index: i32,
    pub hash: String,
    pub pallet: String,
    pub call: String,
    pub signer: Option<String>,
    pub success: bool,
    pub fee: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Event data.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Event {
    pub id: i64,
    pub block_number: i64,
    pub extrinsic_index: Option<i32>,
    pub event_index: i32,
    pub pallet: String,
    pub variant: String,
    pub data: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// Comit transaction data.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ComitTransaction {
    pub id: i64,
    pub block_number: i64,
    pub extrinsic_index: i32,
    pub comit_hash: String,
    pub origin: String,
    pub evm_payload_size: i32,
    pub svm_payload_size: i32,
    pub evm_gas_used: Option<i64>,
    pub svm_compute_used: Option<i64>,
    pub fee_paid: String,
    pub success: bool,
    pub evm_success: Option<bool>,
    pub svm_success: Option<bool>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Account data.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Account {
    pub address: String,
    pub native_balance: String,
    pub nonce: i64,
    pub is_authorized: bool,
    pub first_seen_block: i64,
    pub last_seen_block: i64,
    pub total_transactions: i64,
    pub updated_at: DateTime<Utc>,
}

/// Chain statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainStats {
    pub total_blocks: i64,
    pub latest_block: Option<i64>,
    pub total_extrinsics: i64,
    pub total_events: i64,
    pub total_comits: i64,
    pub successful_comits: i64,
    pub failed_comits: i64,
    pub total_accounts: i64,
}

// ============================================================================
// Database Implementation
// ============================================================================

impl Database {
    /// Connect to the database.
    pub async fn connect(config: &DatabaseConfig) -> Result<Self> {
        info!("Connecting to database...");

        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(Duration::from_secs(30))
            .connect(&config.url)
            .await?;

        info!("Database connected");

        Ok(Self { pool })
    }

    /// Get the connection pool.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    // ========================================================================
    // Block queries
    // ========================================================================

    /// Get block by number.
    pub async fn get_block(&self, number: i64) -> Result<Option<Block>> {
        let block: Option<Block> = sqlx::query_as("SELECT * FROM blocks WHERE number = $1")
            .bind(number)
            .fetch_optional(&self.pool)
            .await?;

        Ok(block)
    }

    /// Get block by hash.
    pub async fn get_block_by_hash(&self, hash: &str) -> Result<Option<Block>> {
        let block: Option<Block> = sqlx::query_as("SELECT * FROM blocks WHERE hash = $1")
            .bind(hash)
            .fetch_optional(&self.pool)
            .await?;

        Ok(block)
    }

    /// Get latest block.
    pub async fn get_latest_block(&self) -> Result<Option<Block>> {
        let block: Option<Block> =
            sqlx::query_as("SELECT * FROM blocks ORDER BY number DESC LIMIT 1")
                .fetch_optional(&self.pool)
                .await?;

        Ok(block)
    }

    /// Get recent blocks.
    pub async fn get_recent_blocks(&self, limit: i64, offset: i64) -> Result<Vec<Block>> {
        let blocks: Vec<Block> =
            sqlx::query_as("SELECT * FROM blocks ORDER BY number DESC LIMIT $1 OFFSET $2")
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?;

        Ok(blocks)
    }

    /// Get blocks in range.
    pub async fn get_blocks_range(&self, from: i64, to: i64) -> Result<Vec<Block>> {
        let blocks: Vec<Block> = sqlx::query_as(
            "SELECT * FROM blocks WHERE number >= $1 AND number <= $2 ORDER BY number",
        )
        .bind(from)
        .bind(to)
        .fetch_all(&self.pool)
        .await?;

        Ok(blocks)
    }

    // ========================================================================
    // Extrinsic queries
    // ========================================================================

    /// Get extrinsic by hash.
    pub async fn get_extrinsic(&self, hash: &str) -> Result<Option<Extrinsic>> {
        let ext: Option<Extrinsic> = sqlx::query_as("SELECT * FROM extrinsics WHERE hash = $1")
            .bind(hash)
            .fetch_optional(&self.pool)
            .await?;

        Ok(ext)
    }

    /// Get extrinsics for a block.
    pub async fn get_block_extrinsics(&self, block_number: i64) -> Result<Vec<Extrinsic>> {
        let exts: Vec<Extrinsic> = sqlx::query_as(
            "SELECT * FROM extrinsics WHERE block_number = $1 ORDER BY extrinsic_index",
        )
        .bind(block_number)
        .fetch_all(&self.pool)
        .await?;

        Ok(exts)
    }

    /// Get extrinsics by account.
    pub async fn get_account_extrinsics(
        &self,
        address: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Extrinsic>> {
        let exts: Vec<Extrinsic> = sqlx::query_as(
            "SELECT * FROM extrinsics WHERE signer = $1 ORDER BY id DESC LIMIT $2 OFFSET $3",
        )
        .bind(address)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(exts)
    }

    /// Get recent extrinsics.
    pub async fn get_recent_extrinsics(&self, limit: i64, offset: i64) -> Result<Vec<Extrinsic>> {
        let exts: Vec<Extrinsic> =
            sqlx::query_as("SELECT * FROM extrinsics ORDER BY id DESC LIMIT $1 OFFSET $2")
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?;

        Ok(exts)
    }

    // ========================================================================
    // Event queries
    // ========================================================================

    /// Get events for a block.
    pub async fn get_block_events(&self, block_number: i64) -> Result<Vec<Event>> {
        let events: Vec<Event> =
            sqlx::query_as("SELECT * FROM events WHERE block_number = $1 ORDER BY event_index")
                .bind(block_number)
                .fetch_all(&self.pool)
                .await?;

        Ok(events)
    }

    /// Get events by pallet.
    pub async fn get_events_by_pallet(
        &self,
        pallet: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Event>> {
        let events: Vec<Event> = sqlx::query_as(
            "SELECT * FROM events WHERE pallet = $1 ORDER BY id DESC LIMIT $2 OFFSET $3",
        )
        .bind(pallet)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(events)
    }

    /// Get events by pallet and variant.
    pub async fn get_events_by_type(
        &self,
        pallet: &str,
        variant: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Event>> {
        let events: Vec<Event> = sqlx::query_as(
            "SELECT * FROM events WHERE pallet = $1 AND variant = $2 ORDER BY id DESC LIMIT $3 OFFSET $4"
        )
        .bind(pallet)
        .bind(variant)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(events)
    }

    // ========================================================================
    // Comit queries
    // ========================================================================

    /// Get Comit by hash.
    pub async fn get_comit(&self, hash: &str) -> Result<Option<ComitTransaction>> {
        let comit: Option<ComitTransaction> =
            sqlx::query_as("SELECT * FROM comit_transactions WHERE comit_hash = $1")
                .bind(hash)
                .fetch_optional(&self.pool)
                .await?;

        Ok(comit)
    }

    /// Get recent Comits.
    pub async fn get_recent_comits(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ComitTransaction>> {
        let comits: Vec<ComitTransaction> =
            sqlx::query_as("SELECT * FROM comit_transactions ORDER BY id DESC LIMIT $1 OFFSET $2")
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?;

        Ok(comits)
    }

    /// Get Comits by origin account.
    pub async fn get_account_comits(
        &self,
        origin: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ComitTransaction>> {
        let comits: Vec<ComitTransaction> = sqlx::query_as(
            "SELECT * FROM comit_transactions WHERE origin = $1 ORDER BY id DESC LIMIT $2 OFFSET $3"
        )
        .bind(origin)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(comits)
    }

    // ========================================================================
    // Account queries
    // ========================================================================

    /// Get account by address.
    pub async fn get_account(&self, address: &str) -> Result<Option<Account>> {
        let account: Option<Account> = sqlx::query_as("SELECT * FROM accounts WHERE address = $1")
            .bind(address)
            .fetch_optional(&self.pool)
            .await?;

        Ok(account)
    }

    /// Get top accounts by balance.
    pub async fn get_top_accounts(&self, limit: i64) -> Result<Vec<Account>> {
        let accounts: Vec<Account> = sqlx::query_as(
            "SELECT * FROM accounts ORDER BY CAST(native_balance AS NUMERIC) DESC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(accounts)
    }

    /// Search accounts.
    pub async fn search_accounts(&self, query: &str, limit: i64) -> Result<Vec<Account>> {
        let pattern = format!("{}%", query);
        let accounts: Vec<Account> =
            sqlx::query_as("SELECT * FROM accounts WHERE address LIKE $1 LIMIT $2")
                .bind(&pattern)
                .bind(limit)
                .fetch_all(&self.pool)
                .await?;

        Ok(accounts)
    }

    // ========================================================================
    // Statistics
    // ========================================================================

    /// Get chain statistics.
    pub async fn get_stats(&self) -> Result<ChainStats> {
        let total_blocks: (i64,) = sqlx::query_as("SELECT COUNT(*)::bigint FROM blocks")
            .fetch_one(&self.pool)
            .await?;

        let latest: Option<(i64,)> = sqlx::query_as("SELECT MAX(number) FROM blocks")
            .fetch_optional(&self.pool)
            .await?;

        let total_extrinsics: (i64,) = sqlx::query_as("SELECT COUNT(*)::bigint FROM extrinsics")
            .fetch_one(&self.pool)
            .await?;

        let total_events: (i64,) = sqlx::query_as("SELECT COUNT(*)::bigint FROM events")
            .fetch_one(&self.pool)
            .await?;

        let total_comits: (i64,) =
            sqlx::query_as("SELECT COUNT(*)::bigint FROM comit_transactions")
                .fetch_one(&self.pool)
                .await?;

        let successful_comits: (i64,) =
            sqlx::query_as("SELECT COUNT(*)::bigint FROM comit_transactions WHERE success = true")
                .fetch_one(&self.pool)
                .await?;

        let total_accounts: (i64,) = sqlx::query_as("SELECT COUNT(*)::bigint FROM accounts")
            .fetch_one(&self.pool)
            .await?;

        Ok(ChainStats {
            total_blocks: total_blocks.0,
            latest_block: latest.and_then(|l| Some(l.0)),
            total_extrinsics: total_extrinsics.0,
            total_events: total_events.0,
            total_comits: total_comits.0,
            successful_comits: successful_comits.0,
            failed_comits: total_comits.0 - successful_comits.0,
            total_accounts: total_accounts.0,
        })
    }
}
