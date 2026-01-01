-- Atlas Sphere Indexer Schema
-- Initial migration

-- Indexer state tracking
CREATE TABLE IF NOT EXISTS indexer_state (
    key VARCHAR(255) PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexed blocks
CREATE TABLE IF NOT EXISTS blocks (
    number BIGINT PRIMARY KEY,
    hash VARCHAR(66) NOT NULL UNIQUE,
    parent_hash VARCHAR(66) NOT NULL,
    state_root VARCHAR(66) NOT NULL,
    extrinsics_root VARCHAR(66) NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    author VARCHAR(66),
    extrinsic_count INTEGER NOT NULL DEFAULT 0,
    event_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_blocks_hash ON blocks(hash);
CREATE INDEX IF NOT EXISTS idx_blocks_timestamp ON blocks(timestamp);

-- Indexed extrinsics
CREATE TABLE IF NOT EXISTS extrinsics (
    id BIGSERIAL PRIMARY KEY,
    block_number BIGINT NOT NULL REFERENCES blocks(number),
    extrinsic_index INTEGER NOT NULL,
    hash VARCHAR(66) NOT NULL,
    pallet VARCHAR(64) NOT NULL,
    call VARCHAR(64) NOT NULL,
    signer VARCHAR(66),
    success BOOLEAN NOT NULL DEFAULT true,
    fee VARCHAR(40),
    raw_data BYTEA,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (block_number, extrinsic_index)
);

CREATE INDEX IF NOT EXISTS idx_extrinsics_hash ON extrinsics(hash);
CREATE INDEX IF NOT EXISTS idx_extrinsics_block ON extrinsics(block_number);
CREATE INDEX IF NOT EXISTS idx_extrinsics_signer ON extrinsics(signer);
CREATE INDEX IF NOT EXISTS idx_extrinsics_pallet ON extrinsics(pallet);
CREATE INDEX IF NOT EXISTS idx_extrinsics_pallet_call ON extrinsics(pallet, call);

-- Indexed events
CREATE TABLE IF NOT EXISTS events (
    id BIGSERIAL PRIMARY KEY,
    block_number BIGINT NOT NULL REFERENCES blocks(number),
    extrinsic_index INTEGER,
    event_index INTEGER NOT NULL,
    pallet VARCHAR(64) NOT NULL,
    variant VARCHAR(64) NOT NULL,
    data JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (block_number, event_index)
);

CREATE INDEX IF NOT EXISTS idx_events_block ON events(block_number);
CREATE INDEX IF NOT EXISTS idx_events_pallet ON events(pallet);
CREATE INDEX IF NOT EXISTS idx_events_pallet_variant ON events(pallet, variant);
CREATE INDEX IF NOT EXISTS idx_events_data ON events USING GIN(data);

-- Comit transactions
CREATE TABLE IF NOT EXISTS comit_transactions (
    id BIGSERIAL PRIMARY KEY,
    block_number BIGINT NOT NULL REFERENCES blocks(number),
    extrinsic_index INTEGER NOT NULL,
    comit_hash VARCHAR(66) NOT NULL UNIQUE,
    origin VARCHAR(66) NOT NULL,
    evm_payload_size INTEGER NOT NULL DEFAULT 0,
    svm_payload_size INTEGER NOT NULL DEFAULT 0,
    evm_gas_used BIGINT,
    svm_compute_used BIGINT,
    fee_paid VARCHAR(40) NOT NULL,
    success BOOLEAN NOT NULL,
    evm_success BOOLEAN,
    svm_success BOOLEAN,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_comit_hash ON comit_transactions(comit_hash);
CREATE INDEX IF NOT EXISTS idx_comit_block ON comit_transactions(block_number);
CREATE INDEX IF NOT EXISTS idx_comit_origin ON comit_transactions(origin);
CREATE INDEX IF NOT EXISTS idx_comit_success ON comit_transactions(success);

-- Indexed accounts
CREATE TABLE IF NOT EXISTS accounts (
    address VARCHAR(66) PRIMARY KEY,
    native_balance VARCHAR(40) NOT NULL DEFAULT '0',
    nonce BIGINT NOT NULL DEFAULT 0,
    is_authorized BOOLEAN NOT NULL DEFAULT false,
    first_seen_block BIGINT NOT NULL,
    last_seen_block BIGINT NOT NULL,
    total_transactions BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_accounts_balance ON accounts(native_balance);
CREATE INDEX IF NOT EXISTS idx_accounts_authorized ON accounts(is_authorized);

-- Asset balances
CREATE TABLE IF NOT EXISTS asset_balances (
    id BIGSERIAL PRIMARY KEY,
    account VARCHAR(66) NOT NULL REFERENCES accounts(address),
    asset_id VARCHAR(66) NOT NULL,
    balance VARCHAR(40) NOT NULL DEFAULT '0',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (account, asset_id)
);

CREATE INDEX IF NOT EXISTS idx_asset_balances_account ON asset_balances(account);
CREATE INDEX IF NOT EXISTS idx_asset_balances_asset ON asset_balances(asset_id);

-- EVM logs
CREATE TABLE IF NOT EXISTS evm_logs (
    id BIGSERIAL PRIMARY KEY,
    block_number BIGINT NOT NULL REFERENCES blocks(number),
    transaction_index INTEGER NOT NULL,
    log_index INTEGER NOT NULL,
    contract_address VARCHAR(42) NOT NULL,
    topic0 VARCHAR(66),
    topic1 VARCHAR(66),
    topic2 VARCHAR(66),
    topic3 VARCHAR(66),
    data BYTEA NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_evm_logs_block ON evm_logs(block_number);
CREATE INDEX IF NOT EXISTS idx_evm_logs_contract ON evm_logs(contract_address);
CREATE INDEX IF NOT EXISTS idx_evm_logs_topic0 ON evm_logs(topic0);
CREATE INDEX IF NOT EXISTS idx_evm_logs_topic1 ON evm_logs(topic1);

-- SVM instructions
CREATE TABLE IF NOT EXISTS svm_instructions (
    id BIGSERIAL PRIMARY KEY,
    block_number BIGINT NOT NULL REFERENCES blocks(number),
    transaction_index INTEGER NOT NULL,
    instruction_index INTEGER NOT NULL,
    program_id VARCHAR(66) NOT NULL,
    accounts JSONB NOT NULL DEFAULT '[]',
    data BYTEA NOT NULL DEFAULT '',
    success BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_svm_instructions_block ON svm_instructions(block_number);
CREATE INDEX IF NOT EXISTS idx_svm_instructions_program ON svm_instructions(program_id);

-- Views for common queries

-- Recent blocks view
CREATE OR REPLACE VIEW recent_blocks AS
SELECT 
    number,
    hash,
    timestamp,
    extrinsic_count,
    event_count
FROM blocks
ORDER BY number DESC
LIMIT 100;

-- Recent Comits view
CREATE OR REPLACE VIEW recent_comits AS
SELECT 
    c.comit_hash,
    c.origin,
    c.success,
    c.evm_payload_size,
    c.svm_payload_size,
    c.fee_paid,
    b.number as block_number,
    b.timestamp
FROM comit_transactions c
JOIN blocks b ON c.block_number = b.number
ORDER BY b.number DESC, c.extrinsic_index DESC
LIMIT 100;

-- Statistics view
CREATE OR REPLACE VIEW indexer_stats AS
SELECT
    (SELECT COUNT(*) FROM blocks) as total_blocks,
    (SELECT MAX(number) FROM blocks) as latest_block,
    (SELECT COUNT(*) FROM extrinsics) as total_extrinsics,
    (SELECT COUNT(*) FROM events) as total_events,
    (SELECT COUNT(*) FROM comit_transactions) as total_comits,
    (SELECT COUNT(*) FROM accounts) as total_accounts,
    (SELECT COUNT(*) FROM comit_transactions WHERE success = true) as successful_comits,
    (SELECT COUNT(*) FROM comit_transactions WHERE success = false) as failed_comits;
