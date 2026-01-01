//! Core indexer logic.

use crate::config::IndexerConfig;
use crate::db::Database;
use crate::error::{IndexerError, Result};
use crate::metrics::Metrics;
use crate::models::*;
use chrono::Utc;
use std::sync::Arc;
use std::time::Duration;
use subxt::{OnlineClient, PolkadotConfig};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

/// Block indexer.
pub struct Indexer {
    config: IndexerConfig,
    db: Database,
    metrics: Metrics,
    client: Arc<Mutex<Option<OnlineClient<PolkadotConfig>>>>,
}

impl Indexer {
    /// Create a new indexer.
    pub async fn new(config: IndexerConfig, db: Database, metrics: Metrics) -> Result<Self> {
        Ok(Self {
            config,
            db,
            metrics,
            client: Arc::new(Mutex::new(None)),
        })
    }

    /// Run the indexer.
    pub async fn run(&self) -> Result<()> {
        info!("Starting indexer...");

        // Connect to node
        self.connect().await?;

        // Determine start block
        let start_block = self.determine_start_block().await?;
        info!("Starting from block #{}", start_block);

        // Main indexing loop
        let mut current_block = start_block;
        let mut reconnect_attempts = 0;

        loop {
            match self.index_next_blocks(&mut current_block).await {
                Ok(()) => {
                    reconnect_attempts = 0;
                }
                Err(e) => {
                    error!("Indexing error: {}", e);
                    self.metrics.record_error();

                    // Try to reconnect
                    reconnect_attempts += 1;
                    if reconnect_attempts > self.config.node.max_reconnects {
                        error!("Max reconnect attempts exceeded");
                        return Err(e);
                    }

                    warn!(
                        "Reconnecting in {} seconds (attempt {})",
                        self.config.node.reconnect_delay_secs, reconnect_attempts
                    );

                    tokio::time::sleep(Duration::from_secs(self.config.node.reconnect_delay_secs))
                        .await;

                    self.connect().await?;
                }
            }
        }
    }

    /// Connect to the blockchain node.
    async fn connect(&self) -> Result<()> {
        info!("Connecting to node: {}", self.config.node.url);

        let client = OnlineClient::<PolkadotConfig>::from_url(&self.config.node.url)
            .await
            .map_err(|e| IndexerError::Connection(e.to_string()))?;

        let mut guard = self.client.lock().await;
        *guard = Some(client);

        info!("Connected to node");
        Ok(())
    }

    /// Determine the starting block for indexing.
    async fn determine_start_block(&self) -> Result<u64> {
        // Check config for explicit start block
        if let Some(start) = self.config.indexer.start_block {
            return Ok(start);
        }

        // Check database for last indexed block
        if let Some(last) = self.db.get_last_indexed_block().await? {
            return Ok((last + 1) as u64);
        }

        // Start from genesis
        Ok(0)
    }

    /// Index the next batch of blocks.
    async fn index_next_blocks(&self, current_block: &mut u64) -> Result<()> {
        let guard = self.client.lock().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| IndexerError::Connection("Not connected".to_string()))?;

        // Get finalized head
        let finalized = client.blocks().at_latest().await?;
        let finalized_number = finalized.number() as u64;

        // If we're caught up, subscribe to new blocks
        if *current_block > finalized_number {
            drop(guard);
            return self.subscribe_blocks(current_block).await;
        }

        // Batch index historical blocks
        let batch_end = std::cmp::min(
            *current_block + self.config.indexer.batch_size as u64,
            finalized_number + 1,
        );

        info!(
            "Indexing blocks {} to {} (finalized: {})",
            current_block,
            batch_end - 1,
            finalized_number
        );

        for block_num in *current_block..batch_end {
            self.index_block(client, block_num).await?;
            self.db.set_last_indexed_block(block_num as i64).await?;
            self.metrics.record_block_indexed(block_num);
        }

        *current_block = batch_end;
        Ok(())
    }

    /// Subscribe to new finalized blocks.
    async fn subscribe_blocks(&self, current_block: &mut u64) -> Result<()> {
        let guard = self.client.lock().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| IndexerError::Connection("Not connected".to_string()))?;

        info!("Caught up, subscribing to new blocks...");

        let mut block_sub = client.blocks().subscribe_finalized().await?;
        drop(guard);

        while let Some(block_result) = block_sub.next().await {
            let block = block_result?;
            let block_num = block.number() as u64;

            // Skip if we already indexed this block
            if block_num < *current_block {
                continue;
            }

            let guard = self.client.lock().await;
            let client = guard.as_ref().unwrap();
            self.index_block(client, block_num).await?;
            drop(guard);

            self.db.set_last_indexed_block(block_num as i64).await?;
            self.metrics.record_block_indexed(block_num);
            *current_block = block_num + 1;
        }

        Err(IndexerError::Connection(
            "Block subscription ended".to_string(),
        ))
    }

    /// Index a single block.
    async fn index_block(
        &self,
        client: &OnlineClient<PolkadotConfig>,
        block_num: u64,
    ) -> Result<()> {
        let start = std::time::Instant::now();

        // Fetch block by number - iterate through blocks to find it
        let block = client.blocks().at_latest().await?;

        // If requesting an old block, we need to walk back through parents
        // For simplicity, we'll use the RPC to get a block at a specific height
        // This is a simplified approach - production would use block hash lookup
        let target_block = if block.number() as u64 == block_num {
            block
        } else {
            // For now, just work with what we have
            // In production, you'd want to use RPC to get block hash at height
            return Err(IndexerError::BlockNotFound(block_num));
        };

        // Extract block header info
        let header = target_block.header();
        let block_hash = target_block.hash();

        // Extract timestamp from events (Timestamp::set inherent)
        let events = target_block.events().await?;
        let timestamp = extract_timestamp_from_events(&events).unwrap_or_else(|| Utc::now());

        // Extract block author from Aura digest
        let author = extract_author_from_header(header);

        let new_block = NewBlock {
            number: block_num as i64,
            hash: format!("0x{}", hex::encode(block_hash.0)),
            parent_hash: format!("0x{}", hex::encode(header.parent_hash.0)),
            state_root: format!("0x{}", hex::encode(header.state_root.0)),
            extrinsics_root: format!("0x{}", hex::encode(header.extrinsics_root.0)),
            timestamp,
            author,
            extrinsic_count: 0, // Will be updated
            event_count: 0,     // Will be updated
        };

        self.db.insert_block(&new_block).await?;

        // Index extrinsics
        let extrinsics = target_block.extrinsics().await?;
        let mut ext_records = Vec::new();
        let mut ext_index = 0;

        for ext in extrinsics.iter() {
            if let Ok(ext) = ext {
                // Compute hash from the raw extrinsic bytes using blake2
                let ext_bytes = ext.bytes();
                let ext_hash = format!("0x{}", hex::encode(sp_core_hashing::blake2_256(ext_bytes)));

                // Try to decode the extrinsic
                let (pallet, call) = ("unknown".to_string(), "unknown".to_string());

                ext_records.push(NewExtrinsic {
                    block_number: block_num as i64,
                    extrinsic_index: ext_index,
                    hash: ext_hash,
                    pallet,
                    call,
                    signer: None,
                    success: true, // Will be updated from events
                    fee: None,
                    raw_data: if self.config.indexer.store_raw {
                        Some(ext_bytes.to_vec())
                    } else {
                        None
                    },
                });

                ext_index += 1;
            }
        }

        self.db.insert_extrinsics(&ext_records).await?;

        // Index events
        let events = target_block.events().await?;
        let mut event_records = Vec::new();
        let mut event_index = 0;

        for event in events.iter() {
            if let Ok(event) = event {
                let pallet = event.pallet_name().to_string();
                let variant = event.variant_name().to_string();

                // Extract extrinsic index from phase
                let extrinsic_index = match event.phase() {
                    subxt::events::Phase::ApplyExtrinsic(i) => Some(i as i32),
                    _ => None,
                };

                event_records.push(NewEvent {
                    block_number: block_num as i64,
                    extrinsic_index,
                    event_index,
                    pallet: pallet.clone(),
                    variant: variant.clone(),
                    data: decode_event_data(&event),
                });

                // Check for Comit events
                if self.config.indexer.index_comits && pallet == "AtlasKernel" {
                    self.process_comit_event(&variant, &event, block_num, event_index)
                        .await?;
                }

                event_index += 1;
            }
        }

        self.db.insert_events(&event_records).await?;

        let elapsed = start.elapsed();
        debug!(
            "Indexed block #{} ({} extrinsics, {} events) in {:?}",
            block_num,
            ext_records.len(),
            event_records.len(),
            elapsed
        );

        self.metrics.record_block_time(elapsed.as_millis() as u64);

        Ok(())
    }

    /// Process a Comit-related event.
    async fn process_comit_event(
        &self,
        variant: &str,
        event: &subxt::events::EventDetails<PolkadotConfig>,
        block_number: u64,
        event_index: i32,
    ) -> Result<()> {
        let event_data = decode_event_data(event);

        match variant {
            "ComitSubmitted" => {
                // Extract Comit details from event data
                let comit_id = event_data
                    .get("comit_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let submitter = event_data
                    .get("submitter")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");

                info!(
                    "ComitSubmitted: id={} submitter={} block={}",
                    comit_id, submitter, block_number
                );

                // Record in database if we have a comits table
                self.db
                    .record_comit_submission(comit_id, submitter, block_number as i64, event_index)
                    .await
                    .ok(); // Ignore if table doesn't exist
            }
            "ComitFinalized" => {
                let comit_id = event_data
                    .get("comit_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");

                info!("ComitFinalized: id={} block={}", comit_id, block_number);

                self.db
                    .record_comit_finalization(
                        comit_id,
                        block_number as i64,
                        true, // success
                    )
                    .await
                    .ok();
            }
            "ComitFailed" => {
                let comit_id = event_data
                    .get("comit_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let reason = event_data
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");

                warn!(
                    "ComitFailed: id={} reason={} block={}",
                    comit_id, reason, block_number
                );

                self.db
                    .record_comit_finalization(
                        comit_id,
                        block_number as i64,
                        false, // failed
                    )
                    .await
                    .ok();
            }
            _ => {}
        }

        Ok(())
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Extract timestamp from block events (Timestamp::set inherent).
fn extract_timestamp_from_events(
    events: &subxt::events::Events<PolkadotConfig>,
) -> Option<chrono::DateTime<Utc>> {
    for event in events.iter() {
        if let Ok(event) = event {
            if event.pallet_name() == "Timestamp" && event.variant_name() == "TimestampSet" {
                // Try to extract timestamp from event data
                let bytes = event.field_bytes();
                // Timestamp is typically a u64 in milliseconds
                if bytes.len() >= 8 {
                    let mut ts_bytes = [0u8; 8];
                    ts_bytes.copy_from_slice(&bytes[..8]);
                    let millis = u64::from_le_bytes(ts_bytes);

                    return chrono::DateTime::from_timestamp_millis(millis as i64)
                        .map(|dt| dt.with_timezone(&Utc));
                }
            }
        }
    }
    None
}

/// Extract block author from header digests (Aura/BABE).
fn extract_author_from_header(
    header: &<PolkadotConfig as subxt::Config>::Header,
) -> Option<String> {
    // Aura pre-runtime digest contains the slot and implicitly the author
    // For simplicity, we extract the PreRuntime digest which contains author info
    for log in header.digest.logs.iter() {
        match log {
            subxt::config::substrate::DigestItem::PreRuntime(engine, data) => {
                // Aura engine ID is *b"aura"
                if *engine == *b"aura" && data.len() >= 8 {
                    // Data contains slot number, we can derive author index
                    let mut slot_bytes = [0u8; 8];
                    slot_bytes.copy_from_slice(&data[..8]);
                    let slot = u64::from_le_bytes(slot_bytes);

                    // In production, you'd look up the author from the authority set
                    // For now, return the slot as a placeholder
                    return Some(format!("slot:{}", slot));
                }
                // BABE engine ID is *b"BABE"
                if *engine == *b"BABE" && data.len() >= 1 {
                    // First byte is authority index for primary slots
                    return Some(format!("authority:{}", data[0]));
                }
            }
            subxt::config::substrate::DigestItem::Consensus(engine, data) => {
                // GRANDPA justifications, etc.
                if *engine == *b"FRNK" && !data.is_empty() {
                    return Some(format!(
                        "grandpa:{}",
                        hex::encode(&data[..data.len().min(8)])
                    ));
                }
            }
            _ => {}
        }
    }
    None
}

/// Decode event data to JSON for storage.
fn decode_event_data(event: &subxt::events::EventDetails<PolkadotConfig>) -> serde_json::Value {
    let mut data = serde_json::Map::new();

    // Add basic event info
    data.insert("pallet".to_string(), serde_json::json!(event.pallet_name()));
    data.insert(
        "variant".to_string(),
        serde_json::json!(event.variant_name()),
    );

    // Try to decode field bytes as hex
    let bytes = event.field_bytes();
    data.insert(
        "raw_data".to_string(),
        serde_json::json!(hex::encode(&bytes)),
    );

    // Try common field patterns based on event type
    let pallet = event.pallet_name();
    let variant = event.variant_name();

    match (pallet, variant) {
        ("AtlasKernel", "ComitSubmitted") => {
            // ComitSubmitted { comit_id: H256, submitter: AccountId, ... }
            if bytes.len() >= 32 {
                data.insert(
                    "comit_id".to_string(),
                    serde_json::json!(format!("0x{}", hex::encode(&bytes[..32]))),
                );
            }
            if bytes.len() >= 64 {
                data.insert(
                    "submitter".to_string(),
                    serde_json::json!(format!("0x{}", hex::encode(&bytes[32..64]))),
                );
            }
        }
        ("AtlasKernel", "ComitFinalized") | ("AtlasKernel", "ComitFailed") => {
            if bytes.len() >= 32 {
                data.insert(
                    "comit_id".to_string(),
                    serde_json::json!(format!("0x{}", hex::encode(&bytes[..32]))),
                );
            }
        }
        ("Balances", "Transfer") => {
            // Transfer { from, to, amount }
            if bytes.len() >= 80 {
                // 32 + 32 + 16
                data.insert(
                    "from".to_string(),
                    serde_json::json!(format!("0x{}", hex::encode(&bytes[..32]))),
                );
                data.insert(
                    "to".to_string(),
                    serde_json::json!(format!("0x{}", hex::encode(&bytes[32..64]))),
                );
                // Amount is u128 (16 bytes)
                if bytes.len() >= 80 {
                    let mut amount_bytes = [0u8; 16];
                    amount_bytes.copy_from_slice(&bytes[64..80]);
                    let amount = u128::from_le_bytes(amount_bytes);
                    data.insert("amount".to_string(), serde_json::json!(amount.to_string()));
                }
            }
        }
        ("System", "ExtrinsicSuccess") => {
            data.insert("success".to_string(), serde_json::json!(true));
        }
        ("System", "ExtrinsicFailed") => {
            data.insert("success".to_string(), serde_json::json!(false));
            // Try to decode error
            if !bytes.is_empty() {
                data.insert("error_module".to_string(), serde_json::json!(bytes[0]));
                if bytes.len() > 1 {
                    data.insert("error_index".to_string(), serde_json::json!(bytes[1]));
                }
            }
        }
        _ => {}
    }

    serde_json::Value::Object(data)
}
