//! Database connection and operations.
//!
//! Uses runtime queries (not compile-time checked) to avoid requiring DATABASE_URL
//! at build time.

use crate::config::DatabaseConfig;
use crate::error::{IndexerError, Result};
use crate::models::*;
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;
use tracing::{debug, info};

/// Database connection pool wrapper.
#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    /// Connect to the database.
    pub async fn connect(config: &DatabaseConfig) -> Result<Self> {
        info!("Connecting to database...");

        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(Duration::from_secs(config.acquire_timeout_secs))
            .connect(&config.url)
            .await?;

        info!("Database connected");

        Ok(Self { pool })
    }

    /// Run database migrations.
    pub async fn migrate(&self) -> Result<()> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|e| IndexerError::Database(e.into()))?;
        Ok(())
    }

    /// Get the connection pool.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    // =========================================================================
    // State Management
    // =========================================================================

    /// Get indexer state value.
    pub async fn get_state(&self, key: &str) -> Result<Option<String>> {
        let result: Option<(String,)> =
            sqlx::query_as("SELECT value FROM indexer_state WHERE key = $1")
                .bind(key)
                .fetch_optional(&self.pool)
                .await?;

        Ok(result.map(|r| r.0))
    }

    /// Set indexer state value.
    pub async fn set_state(&self, key: &str, value: &str) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO indexer_state (key, value, updated_at)
            VALUES ($1, $2, NOW())
            ON CONFLICT (key) DO UPDATE SET value = $2, updated_at = NOW()
            "#,
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get last indexed block number.
    pub async fn get_last_indexed_block(&self) -> Result<Option<i64>> {
        let result = self.get_state("last_indexed_block").await?;
        Ok(result.and_then(|s| s.parse().ok()))
    }

    /// Set last indexed block number.
    pub async fn set_last_indexed_block(&self, block: i64) -> Result<()> {
        self.set_state("last_indexed_block", &block.to_string())
            .await
    }

    // =========================================================================
    // Block Operations
    // =========================================================================

    /// Insert a new block.
    pub async fn insert_block(&self, block: &NewBlock) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO blocks (
                number, hash, parent_hash, state_root, extrinsics_root,
                timestamp, author, extrinsic_count, event_count, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
            ON CONFLICT (number) DO NOTHING
            "#,
        )
        .bind(block.number)
        .bind(&block.hash)
        .bind(&block.parent_hash)
        .bind(&block.state_root)
        .bind(&block.extrinsics_root)
        .bind(block.timestamp)
        .bind(&block.author)
        .bind(block.extrinsic_count)
        .bind(block.event_count)
        .execute(&self.pool)
        .await?;

        debug!("Inserted block #{}", block.number);
        Ok(())
    }

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

    // =========================================================================
    // Extrinsic Operations
    // =========================================================================

    /// Insert extrinsics in batch.
    pub async fn insert_extrinsics(&self, extrinsics: &[NewExtrinsic]) -> Result<()> {
        if extrinsics.is_empty() {
            return Ok(());
        }

        for ext in extrinsics {
            sqlx::query(
                r#"
                INSERT INTO extrinsics (
                    block_number, extrinsic_index, hash, pallet, call,
                    signer, success, fee, raw_data, created_at
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
                ON CONFLICT (block_number, extrinsic_index) DO NOTHING
                "#,
            )
            .bind(ext.block_number)
            .bind(ext.extrinsic_index)
            .bind(&ext.hash)
            .bind(&ext.pallet)
            .bind(&ext.call)
            .bind(&ext.signer)
            .bind(ext.success)
            .bind(&ext.fee)
            .bind(&ext.raw_data)
            .execute(&self.pool)
            .await?;
        }

        debug!("Inserted {} extrinsics", extrinsics.len());
        Ok(())
    }

    /// Get extrinsics for a block.
    pub async fn get_extrinsics(&self, block_number: i64) -> Result<Vec<Extrinsic>> {
        let extrinsics: Vec<Extrinsic> = sqlx::query_as(
            "SELECT * FROM extrinsics WHERE block_number = $1 ORDER BY extrinsic_index",
        )
        .bind(block_number)
        .fetch_all(&self.pool)
        .await?;

        Ok(extrinsics)
    }

    /// Get extrinsic by hash.
    pub async fn get_extrinsic_by_hash(&self, hash: &str) -> Result<Option<Extrinsic>> {
        let ext: Option<Extrinsic> = sqlx::query_as("SELECT * FROM extrinsics WHERE hash = $1")
            .bind(hash)
            .fetch_optional(&self.pool)
            .await?;

        Ok(ext)
    }

    // =========================================================================
    // Event Operations
    // =========================================================================

    /// Insert events in batch.
    pub async fn insert_events(&self, events: &[NewEvent]) -> Result<()> {
        if events.is_empty() {
            return Ok(());
        }

        for event in events {
            sqlx::query(
                r#"
                INSERT INTO events (
                    block_number, extrinsic_index, event_index, pallet, variant,
                    data, created_at
                ) VALUES ($1, $2, $3, $4, $5, $6, NOW())
                ON CONFLICT (block_number, event_index) DO NOTHING
                "#,
            )
            .bind(event.block_number)
            .bind(event.extrinsic_index)
            .bind(event.event_index)
            .bind(&event.pallet)
            .bind(&event.variant)
            .bind(&event.data)
            .execute(&self.pool)
            .await?;
        }

        debug!("Inserted {} events", events.len());
        Ok(())
    }

    /// Get events for a block.
    pub async fn get_events(&self, block_number: i64) -> Result<Vec<Event>> {
        let events: Vec<Event> =
            sqlx::query_as("SELECT * FROM events WHERE block_number = $1 ORDER BY event_index")
                .bind(block_number)
                .fetch_all(&self.pool)
                .await?;

        Ok(events)
    }

    // =========================================================================
    // Comit Operations
    // =========================================================================

    /// Insert Comit transaction.
    pub async fn insert_comit(&self, comit: &NewComitTransaction) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO comit_transactions (
                block_number, extrinsic_index, comit_hash, origin,
                evm_payload_size, svm_payload_size, evm_gas_used, svm_compute_used,
                fee_paid, success, evm_success, svm_success, error_message, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, NOW())
            ON CONFLICT (comit_hash) DO NOTHING
            "#,
        )
        .bind(comit.block_number)
        .bind(comit.extrinsic_index)
        .bind(&comit.comit_hash)
        .bind(&comit.origin)
        .bind(comit.evm_payload_size)
        .bind(comit.svm_payload_size)
        .bind(comit.evm_gas_used)
        .bind(comit.svm_compute_used)
        .bind(&comit.fee_paid)
        .bind(comit.success)
        .bind(comit.evm_success)
        .bind(comit.svm_success)
        .bind(&comit.error_message)
        .execute(&self.pool)
        .await?;

        debug!("Inserted Comit {}", comit.comit_hash);
        Ok(())
    }

    /// Get Comit transactions for a block.
    pub async fn get_comits(&self, block_number: i64) -> Result<Vec<ComitTransaction>> {
        let comits: Vec<ComitTransaction> =
            sqlx::query_as("SELECT * FROM comit_transactions WHERE block_number = $1")
                .bind(block_number)
                .fetch_all(&self.pool)
                .await?;

        Ok(comits)
    }

    /// Get Comit by hash.
    pub async fn get_comit_by_hash(&self, hash: &str) -> Result<Option<ComitTransaction>> {
        let comit: Option<ComitTransaction> =
            sqlx::query_as("SELECT * FROM comit_transactions WHERE comit_hash = $1")
                .bind(hash)
                .fetch_optional(&self.pool)
                .await?;

        Ok(comit)
    }

    // =========================================================================
    // Account Operations
    // =========================================================================

    /// Upsert account.
    pub async fn upsert_account(
        &self,
        address: &str,
        balance: &str,
        nonce: i64,
        is_authorized: bool,
        block_number: i64,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO accounts (
                address, native_balance, nonce, is_authorized,
                first_seen_block, last_seen_block, total_transactions, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $5, 1, NOW())
            ON CONFLICT (address) DO UPDATE SET
                native_balance = $2,
                nonce = $3,
                is_authorized = $4,
                last_seen_block = $5,
                total_transactions = accounts.total_transactions + 1,
                updated_at = NOW()
            "#,
        )
        .bind(address)
        .bind(balance)
        .bind(nonce)
        .bind(is_authorized)
        .bind(block_number)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get account by address.
    pub async fn get_account(&self, address: &str) -> Result<Option<Account>> {
        let account: Option<Account> = sqlx::query_as("SELECT * FROM accounts WHERE address = $1")
            .bind(address)
            .fetch_optional(&self.pool)
            .await?;

        Ok(account)
    }

    // =========================================================================
    // Statistics
    // =========================================================================

    /// Get total block count.
    pub async fn get_block_count(&self) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*)::bigint FROM blocks")
            .fetch_one(&self.pool)
            .await?;

        Ok(row.0)
    }

    /// Get total extrinsic count.
    pub async fn get_extrinsic_count(&self) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*)::bigint FROM extrinsics")
            .fetch_one(&self.pool)
            .await?;

        Ok(row.0)
    }

    /// Get total Comit count.
    pub async fn get_comit_count(&self) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*)::bigint FROM comit_transactions")
            .fetch_one(&self.pool)
            .await?;

        Ok(row.0)
    }

    // =========================================================================
    // Comit Event Recording
    // =========================================================================

    /// Record a Comit submission event.
    pub async fn record_comit_submission(
        &self,
        comit_id: &str,
        submitter: &str,
        block_number: i64,
        event_index: i32,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO comit_transactions (comit_id, submitter, block_number, event_index, status, created_at)
            VALUES ($1, $2, $3, $4, 'submitted', NOW())
            ON CONFLICT (comit_id) DO UPDATE SET 
                submitter = $2,
                block_number = $3,
                event_index = $4,
                status = 'submitted',
                updated_at = NOW()
            "#
        )
        .bind(comit_id)
        .bind(submitter)
        .bind(block_number)
        .bind(event_index)
        .execute(&self.pool)
        .await?;

        debug!("Recorded Comit submission: {}", comit_id);
        Ok(())
    }

    /// Record a Comit finalization (success or failure).
    pub async fn record_comit_finalization(
        &self,
        comit_id: &str,
        block_number: i64,
        success: bool,
    ) -> Result<()> {
        let status = if success { "finalized" } else { "failed" };

        sqlx::query(
            r#"
            UPDATE comit_transactions 
            SET status = $2, finalized_at_block = $3, updated_at = NOW()
            WHERE comit_id = $1
            "#,
        )
        .bind(comit_id)
        .bind(status)
        .bind(block_number)
        .execute(&self.pool)
        .await?;

        debug!("Recorded Comit finalization: {} -> {}", comit_id, status);
        Ok(())
    }
}
