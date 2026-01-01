//! GraphQL schema and resolvers.

use crate::db::{Account, Block, ChainStats, ComitTransaction, Database, Event, Extrinsic};
use async_graphql::{Context, EmptyMutation, EmptySubscription, Object, Schema};

/// GraphQL query root.
pub struct QueryRoot;

#[Object]
impl QueryRoot {
    // ========================================================================
    // Block queries
    // ========================================================================

    /// Get block by number.
    async fn block(&self, ctx: &Context<'_>, number: i64) -> async_graphql::Result<Option<Block>> {
        let db = ctx.data::<Database>()?;
        Ok(db.get_block(number).await?)
    }

    /// Get block by hash.
    async fn block_by_hash(
        &self,
        ctx: &Context<'_>,
        hash: String,
    ) -> async_graphql::Result<Option<Block>> {
        let db = ctx.data::<Database>()?;
        Ok(db.get_block_by_hash(&hash).await?)
    }

    /// Get latest block.
    async fn latest_block(&self, ctx: &Context<'_>) -> async_graphql::Result<Option<Block>> {
        let db = ctx.data::<Database>()?;
        Ok(db.get_latest_block().await?)
    }

    /// Get recent blocks.
    async fn blocks(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 20)] limit: i64,
        #[graphql(default = 0)] offset: i64,
    ) -> async_graphql::Result<Vec<Block>> {
        let db = ctx.data::<Database>()?;
        Ok(db.get_recent_blocks(limit.min(100), offset).await?)
    }

    /// Get blocks in a range.
    async fn blocks_range(
        &self,
        ctx: &Context<'_>,
        from: i64,
        to: i64,
    ) -> async_graphql::Result<Vec<Block>> {
        let db = ctx.data::<Database>()?;
        // Limit range to 100 blocks
        let limited_to = (from + 100).min(to);
        Ok(db.get_blocks_range(from, limited_to).await?)
    }

    // ========================================================================
    // Extrinsic queries
    // ========================================================================

    /// Get extrinsic by hash.
    async fn extrinsic(
        &self,
        ctx: &Context<'_>,
        hash: String,
    ) -> async_graphql::Result<Option<Extrinsic>> {
        let db = ctx.data::<Database>()?;
        Ok(db.get_extrinsic(&hash).await?)
    }

    /// Get extrinsics for a block.
    async fn block_extrinsics(
        &self,
        ctx: &Context<'_>,
        block_number: i64,
    ) -> async_graphql::Result<Vec<Extrinsic>> {
        let db = ctx.data::<Database>()?;
        Ok(db.get_block_extrinsics(block_number).await?)
    }

    /// Get recent extrinsics.
    async fn extrinsics(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 20)] limit: i64,
        #[graphql(default = 0)] offset: i64,
    ) -> async_graphql::Result<Vec<Extrinsic>> {
        let db = ctx.data::<Database>()?;
        Ok(db.get_recent_extrinsics(limit.min(100), offset).await?)
    }

    /// Get extrinsics by account.
    async fn account_extrinsics(
        &self,
        ctx: &Context<'_>,
        address: String,
        #[graphql(default = 20)] limit: i64,
        #[graphql(default = 0)] offset: i64,
    ) -> async_graphql::Result<Vec<Extrinsic>> {
        let db = ctx.data::<Database>()?;
        Ok(db
            .get_account_extrinsics(&address, limit.min(100), offset)
            .await?)
    }

    // ========================================================================
    // Event queries
    // ========================================================================

    /// Get events for a block.
    async fn block_events(
        &self,
        ctx: &Context<'_>,
        block_number: i64,
    ) -> async_graphql::Result<Vec<Event>> {
        let db = ctx.data::<Database>()?;
        Ok(db.get_block_events(block_number).await?)
    }

    /// Get events by pallet.
    async fn events_by_pallet(
        &self,
        ctx: &Context<'_>,
        pallet: String,
        #[graphql(default = 20)] limit: i64,
        #[graphql(default = 0)] offset: i64,
    ) -> async_graphql::Result<Vec<Event>> {
        let db = ctx.data::<Database>()?;
        Ok(db
            .get_events_by_pallet(&pallet, limit.min(100), offset)
            .await?)
    }

    /// Get events by pallet and variant.
    async fn events_by_type(
        &self,
        ctx: &Context<'_>,
        pallet: String,
        variant: String,
        #[graphql(default = 20)] limit: i64,
        #[graphql(default = 0)] offset: i64,
    ) -> async_graphql::Result<Vec<Event>> {
        let db = ctx.data::<Database>()?;
        Ok(db
            .get_events_by_type(&pallet, &variant, limit.min(100), offset)
            .await?)
    }

    // ========================================================================
    // Comit queries
    // ========================================================================

    /// Get Comit by hash.
    async fn comit(
        &self,
        ctx: &Context<'_>,
        hash: String,
    ) -> async_graphql::Result<Option<ComitTransaction>> {
        let db = ctx.data::<Database>()?;
        Ok(db.get_comit(&hash).await?)
    }

    /// Get recent Comits.
    async fn comits(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 20)] limit: i64,
        #[graphql(default = 0)] offset: i64,
    ) -> async_graphql::Result<Vec<ComitTransaction>> {
        let db = ctx.data::<Database>()?;
        Ok(db.get_recent_comits(limit.min(100), offset).await?)
    }

    /// Get Comits by origin account.
    async fn account_comits(
        &self,
        ctx: &Context<'_>,
        address: String,
        #[graphql(default = 20)] limit: i64,
        #[graphql(default = 0)] offset: i64,
    ) -> async_graphql::Result<Vec<ComitTransaction>> {
        let db = ctx.data::<Database>()?;
        Ok(db
            .get_account_comits(&address, limit.min(100), offset)
            .await?)
    }

    // ========================================================================
    // Account queries
    // ========================================================================

    /// Get account by address.
    async fn account(
        &self,
        ctx: &Context<'_>,
        address: String,
    ) -> async_graphql::Result<Option<Account>> {
        let db = ctx.data::<Database>()?;
        Ok(db.get_account(&address).await?)
    }

    /// Get top accounts by balance.
    async fn top_accounts(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 20)] limit: i64,
    ) -> async_graphql::Result<Vec<Account>> {
        let db = ctx.data::<Database>()?;
        Ok(db.get_top_accounts(limit.min(100)).await?)
    }

    /// Search accounts.
    async fn search_accounts(
        &self,
        ctx: &Context<'_>,
        query: String,
        #[graphql(default = 10)] limit: i64,
    ) -> async_graphql::Result<Vec<Account>> {
        let db = ctx.data::<Database>()?;
        Ok(db.search_accounts(&query, limit.min(50)).await?)
    }

    // ========================================================================
    // Statistics
    // ========================================================================

    /// Get chain statistics.
    async fn stats(&self, ctx: &Context<'_>) -> async_graphql::Result<ChainStats> {
        let db = ctx.data::<Database>()?;
        Ok(db.get_stats().await?)
    }
}

/// GraphQL schema type.
pub type AppSchema = Schema<QueryRoot, EmptyMutation, EmptySubscription>;

/// Create the GraphQL schema.
pub fn create_schema(db: Database) -> AppSchema {
    Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
        .data(db)
        .finish()
}

// ============================================================================
// Object implementations for complex types
// ============================================================================

#[Object]
impl Block {
    async fn number(&self) -> i64 {
        self.number
    }

    async fn hash(&self) -> &str {
        &self.hash
    }

    async fn parent_hash(&self) -> &str {
        &self.parent_hash
    }

    async fn state_root(&self) -> &str {
        &self.state_root
    }

    async fn extrinsics_root(&self) -> &str {
        &self.extrinsics_root
    }

    async fn timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        self.timestamp
    }

    async fn author(&self) -> Option<&str> {
        self.author.as_deref()
    }

    async fn extrinsic_count(&self) -> i32 {
        self.extrinsic_count
    }

    async fn event_count(&self) -> i32 {
        self.event_count
    }
}

#[Object]
impl Extrinsic {
    async fn id(&self) -> i64 {
        self.id
    }

    async fn block_number(&self) -> i64 {
        self.block_number
    }

    async fn extrinsic_index(&self) -> i32 {
        self.extrinsic_index
    }

    async fn hash(&self) -> &str {
        &self.hash
    }

    async fn pallet(&self) -> &str {
        &self.pallet
    }

    async fn call(&self) -> &str {
        &self.call
    }

    async fn signer(&self) -> Option<&str> {
        self.signer.as_deref()
    }

    async fn success(&self) -> bool {
        self.success
    }

    async fn fee(&self) -> Option<&str> {
        self.fee.as_deref()
    }
}

#[Object]
impl Event {
    async fn id(&self) -> i64 {
        self.id
    }

    async fn block_number(&self) -> i64 {
        self.block_number
    }

    async fn extrinsic_index(&self) -> Option<i32> {
        self.extrinsic_index
    }

    async fn event_index(&self) -> i32 {
        self.event_index
    }

    async fn pallet(&self) -> &str {
        &self.pallet
    }

    async fn variant(&self) -> &str {
        &self.variant
    }

    async fn data(&self) -> &serde_json::Value {
        &self.data
    }
}

#[Object]
impl ComitTransaction {
    async fn id(&self) -> i64 {
        self.id
    }

    async fn block_number(&self) -> i64 {
        self.block_number
    }

    async fn comit_hash(&self) -> &str {
        &self.comit_hash
    }

    async fn origin(&self) -> &str {
        &self.origin
    }

    async fn evm_payload_size(&self) -> i32 {
        self.evm_payload_size
    }

    async fn svm_payload_size(&self) -> i32 {
        self.svm_payload_size
    }

    async fn evm_gas_used(&self) -> Option<i64> {
        self.evm_gas_used
    }

    async fn svm_compute_used(&self) -> Option<i64> {
        self.svm_compute_used
    }

    async fn fee_paid(&self) -> &str {
        &self.fee_paid
    }

    async fn success(&self) -> bool {
        self.success
    }

    async fn evm_success(&self) -> Option<bool> {
        self.evm_success
    }

    async fn svm_success(&self) -> Option<bool> {
        self.svm_success
    }

    async fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }
}

#[Object]
impl Account {
    async fn address(&self) -> &str {
        &self.address
    }

    async fn native_balance(&self) -> &str {
        &self.native_balance
    }

    async fn nonce(&self) -> i64 {
        self.nonce
    }

    async fn is_authorized(&self) -> bool {
        self.is_authorized
    }

    async fn first_seen_block(&self) -> i64 {
        self.first_seen_block
    }

    async fn last_seen_block(&self) -> i64 {
        self.last_seen_block
    }

    async fn total_transactions(&self) -> i64 {
        self.total_transactions
    }
}

#[Object]
impl ChainStats {
    async fn total_blocks(&self) -> i64 {
        self.total_blocks
    }

    async fn latest_block(&self) -> Option<i64> {
        self.latest_block
    }

    async fn total_extrinsics(&self) -> i64 {
        self.total_extrinsics
    }

    async fn total_events(&self) -> i64 {
        self.total_events
    }

    async fn total_comits(&self) -> i64 {
        self.total_comits
    }

    async fn successful_comits(&self) -> i64 {
        self.successful_comits
    }

    async fn failed_comits(&self) -> i64 {
        self.failed_comits
    }

    async fn total_accounts(&self) -> i64 {
        self.total_accounts
    }
}
